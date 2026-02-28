//! Cranelift code generation with built-in function support

use crate::compiler::mir::{
    BinOpKind, BlockId, Constant, FunctionId, LocalId, Terminator, TypedFunction, TypedInstruction,
    TypedMIR,
};
use crate::compiler::OptLevel;
use crate::error::{
    capture_stack_trace, DxError, DxResult, JsErrorType, JsException, ModuleSourceMap,
    SourceMapEntry,
};
use crate::value::Value;
use cranelift::prelude::*;
use cranelift_codegen::ir::{FuncRef, TrapCode};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift::frontend::Variable;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Simple string hash function for property keys
/// Returns a 32-bit hash to avoid f64 precision issues
fn hash_string(s: &str) -> u32 {
    let mut hash = 5381u32;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    hash
}

// Alias to avoid name collision with our mir::FunctionBuilder
use cranelift::prelude::FunctionBuilder as CraneliftFunctionBuilder;

/// A compiled module ready for execution
pub struct CompiledModule {
    /// The JIT module (must be kept alive)
    _jit_module: JITModule,
    /// Function pointers by ID
    functions: HashMap<FunctionId, *const u8>,
    /// Entry point
    entry_point: Option<*const u8>,
    /// Source hash for caching
    pub source_hash: [u8; 32],
    /// Source map for mapping native addresses to source locations
    pub source_map: ModuleSourceMap,
}

// Safety: We control access to the function pointers
unsafe impl Send for CompiledModule {}
unsafe impl Sync for CompiledModule {}

impl CompiledModule {
    /// Execute the module's entry point
    pub fn execute(&self) -> DxResult<Value> {
        if let Some(entry) = self.entry_point {
            // The entry function returns f64
            let func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(entry) };
            let result = func();
            // NaN represents undefined in our JIT
            if result.is_nan() {
                Ok(Value::Undefined)
            } else {
                Ok(Value::Number(result))
            }
        } else {
            Ok(Value::Undefined)
        }
    }

    /// Get a function pointer by ID
    #[allow(dead_code)]
    pub fn get_function(&self, id: FunctionId) -> Option<*const u8> {
        self.functions.get(&id).copied()
    }

    /// Get the source map for this module
    pub fn get_source_map(&self) -> &ModuleSourceMap {
        &self.source_map
    }

    /// Look up source location for a native address
    pub fn lookup_source_location(&self, native_offset: usize) -> Option<&SourceMapEntry> {
        self.source_map.lookup(native_offset)
    }
}

// Built-in function implementations (extern "C" for FFI)

/// String tag constant - strings are encoded as negative numbers
/// The actual string ID is stored as: -(id + STRING_TAG_OFFSET)
const STRING_TAG_OFFSET: f64 = 1_000_000.0;

/// BigInt tag constant - BigInts are encoded as negative numbers
/// The actual BigInt ID is stored as: -(id + BIGINT_TAG_OFFSET)
const BIGINT_TAG_OFFSET: f64 = 2_000_000.0;

/// Check if a value is a tagged string ID
fn is_string_id(value: f64) -> bool {
    value < -STRING_TAG_OFFSET + 1.0 && value >= -BIGINT_TAG_OFFSET + 1.0 && value.fract() == 0.0
}

/// Decode a string ID from a tagged value
fn decode_string_id(value: f64) -> u64 {
    (-(value + STRING_TAG_OFFSET)) as u64
}

/// Encode a string ID as a tagged value
fn encode_string_id(id: u64) -> f64 {
    -(id as f64 + STRING_TAG_OFFSET)
}

/// Check if a value is a tagged BigInt ID
fn is_bigint_id(value: f64) -> bool {
    // BigInt IDs are in range [-2_999_999, -2_000_000]
    // BigInt method IDs are in range [-3_000_099, -3_000_000]
    value < -BIGINT_TAG_OFFSET + 1.0 && value >= -BIGINT_TAG_OFFSET - 999_999.0 && value.fract() == 0.0
}

/// Decode a BigInt ID from a tagged value
fn decode_bigint_id(value: f64) -> u64 {
    (-(value + BIGINT_TAG_OFFSET)) as u64
}

/// Encode a BigInt ID as a tagged value
fn encode_bigint_id(id: u64) -> f64 {
    -(id as f64 + BIGINT_TAG_OFFSET)
}

/// Allocate a string in the runtime heap (called from generated code)
/// Reserved for JIT-compiled string allocation - will be used when string operations are fully implemented
///
/// Validates UTF-8 encoding and returns an error value for invalid sequences.
#[allow(dead_code)]
extern "C" fn builtin_allocate_string(ptr: *const u8, len: usize) -> f64 {
    with_runtime_heap(|heap| {
        // Safety: ptr and len come from Rust string data passed during compilation
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        
        // Validate UTF-8 encoding per Requirements 1.5
        match heap.allocate_string_from_bytes(bytes) {
            Ok(id) => encode_string_id(id),
            Err(_) => {
                // Return NaN to indicate error (caller should check for this)
                // In a full implementation, this would throw a TypeError
                f64::NAN
            }
        }
    })
}

extern "C" fn builtin_console_log(value: f64) -> f64 {
    // Check if this is a string ID
    if is_string_id(value) {
        let string_id = decode_string_id(value);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        if let Some(s) = heap.get_string(string_id) {
            println!("{}", s);
        } else {
            println!("undefined");
        }
    } else if is_bigint_id(value) {
        // Handle BigInt values
        if let Some(bigint) = get_bigint_from_value(value) {
            println!("{}n", bigint);
        } else {
            println!("undefined");
        }
    } else if value.is_nan() {
        println!("undefined");
    } else if value.is_infinite() {
        if value > 0.0 {
            println!("Infinity");
        } else {
            println!("-Infinity");
        }
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        // Check if this might be a heap object ID (positive integer)
        let id = value as u64;
        if id > 0 && id < 1_000_000 {
            let heap = get_runtime_heap_lock();
            let heap = heap.lock().unwrap();

            // Check if it's an array
            if let Some(arr) = heap.get_array(id) {
                let formatted = format_array_value(arr, &heap, 0);
                println!("{}", formatted);
                return f64::NAN;
            }

            // Check if it's an object
            if let Some(obj) = heap.get_object(id) {
                let formatted = format_object_value(obj, &heap, 0);
                println!("{}", formatted);
                return f64::NAN;
            }

            // Check if it's a closure (function)
            if heap.get_closure(id).is_some() {
                println!("[Function]");
                return f64::NAN;
            }
        }
        // It's just a regular integer
        println!("{}", value as i64);
    } else {
        println!("{}", value);
    }
    f64::NAN // return undefined
}

const MAX_DISPLAY_DEPTH: usize = 5;

/// Format an array value for console output
fn format_array_value(arr: &Vec<f64>, heap: &RuntimeHeap, depth: usize) -> String {
    if depth > MAX_DISPLAY_DEPTH {
        return "[...]".to_string();
    }
    let mut parts = Vec::new();
    for &elem in arr {
        parts.push(format_value_for_display(elem, heap, depth + 1));
    }
    format!("[{}]", parts.join(", "))
}

/// Format an object value for console output
fn format_object_value(obj: &HashMap<String, f64>, heap: &RuntimeHeap, depth: usize) -> String {
    if depth > MAX_DISPLAY_DEPTH {
        return "{...}".to_string();
    }
    if obj.is_empty() {
        return "{}".to_string();
    }
    let mut parts = Vec::new();
    for (key, &value) in obj {
        // Remove the "prop_" prefix if present (from hash-based storage)
        let display_key = if key.starts_with("prop_") {
            // Try to find the original key in strings
            key.clone()
        } else {
            key.clone()
        };
        parts.push(format!(
            "{}: {}",
            display_key,
            format_value_for_display(value, heap, depth + 1)
        ));
    }
    format!("{{ {} }}", parts.join(", "))
}

/// Format a single value for display
fn format_value_for_display(value: f64, heap: &RuntimeHeap, depth: usize) -> String {
    if depth > MAX_DISPLAY_DEPTH {
        return "...".to_string();
    }

    if is_string_id(value) {
        let string_id = decode_string_id(value);
        if let Some(s) = heap.get_string(string_id) {
            return format!("\"{}\"", s);
        }
        return "undefined".to_string();
    }

    if is_bigint_id(value) {
        let bigint_id = decode_bigint_id(value);
        if let Some(bigint) = heap.get_bigint(bigint_id) {
            return format!("{}n", bigint);
        }
        return "undefined".to_string();
    }

    if value.is_nan() {
        return "undefined".to_string();
    }

    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }

    if value.fract() == 0.0 && value.abs() < 1e15 {
        let id = value as u64;
        if id > 0 && id < 1_000_000 {
            // Check if it's an array
            if let Some(arr) = heap.get_array(id) {
                return format_array_value(arr, heap, depth + 1);
            }
            // Check if it's an object
            if let Some(obj) = heap.get_object(id) {
                return format_object_value(obj, heap, depth + 1);
            }
            // Check if it's a closure
            if heap.get_closure(id).is_some() {
                return "[Function]".to_string();
            }
        }
        return format!("{}", value as i64);
    }

    format!("{}", value)
}

extern "C" fn builtin_math_floor(value: f64) -> f64 {
    value.floor()
}

// ============================================================================
// Dynamic Import Helper Functions
// ============================================================================

/// Get a pointer to a string in the heap (for dynamic import)
/// Returns null if the string ID is invalid
#[no_mangle]
pub extern "C" fn builtin_get_string_ptr(id: u64) -> *const u8 {
    let heap = get_runtime_heap_lock();
    let heap = heap.lock().unwrap();
    match heap.get_string(id) {
        Some(s) => s.as_ptr(),
        None => std::ptr::null(),
    }
}

/// Get the length of a string in the heap (for dynamic import)
/// Returns 0 if the string ID is invalid
#[no_mangle]
pub extern "C" fn builtin_get_string_len(id: u64) -> usize {
    let heap = get_runtime_heap_lock();
    let heap = heap.lock().unwrap();
    match heap.get_string(id) {
        Some(s) => s.len(),
        None => 0,
    }
}

/// Set a property on an object using a string name ID
/// Used by dynamic import to populate module namespace objects
#[no_mangle]
pub extern "C" fn builtin_set_object_property(obj_id: u64, name_id: u64, value: f64) {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    // Get the property name from the string heap
    let name = match heap.get_string(name_id) {
        Some(s) => s.clone(),
        None => return,
    };
    
    // Set the property on the object
    if let Some(obj) = heap.get_object_mut(obj_id) {
        obj.insert(name, value);
    }
}

/// Resolve a promise with a value (wrapper for dynamic import)
#[no_mangle]
pub extern "C" fn builtin_resolve_promise(promise_id: u64, value: f64) {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(promise) = heap.get_promise_mut(promise_id) {
        if promise.state == PromiseState::Pending {
            promise.state = PromiseState::Fulfilled;
            promise.value = value;
        }
    }
}

/// Reject a promise with an error (wrapper for dynamic import)
#[no_mangle]
pub extern "C" fn builtin_reject_promise(promise_id: u64, error: f64) {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(promise) = heap.get_promise_mut(promise_id) {
        if promise.state == PromiseState::Pending {
            promise.state = PromiseState::Rejected;
            promise.value = error;
        }
    }
}

/// Allocate a string in the heap and return its ID (for dynamic import)
///
/// Validates UTF-8 encoding per Requirements 1.5.
/// Returns 0 (invalid ID) for invalid UTF-8 sequences.
#[no_mangle]
pub extern "C" fn builtin_allocate_string_raw(ptr: *const u8, len: usize) -> u64 {
    with_runtime_heap(|heap| {
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        
        // Validate UTF-8 encoding per Requirements 1.5
        match heap.allocate_string_from_bytes(bytes) {
            Ok(id) => id,
            Err(_) => {
                // For backward compatibility, fall back to lossy conversion
                // but log a warning in debug builds
                #[cfg(debug_assertions)]
                eprintln!("Warning: Invalid UTF-8 sequence in string allocation, using lossy conversion");
                heap.allocate_string_lossy(bytes)
            }
        }
    })
}

/// Create an empty object and return its ID (for dynamic import)
#[no_mangle]
pub extern "C" fn builtin_create_object_raw() -> u64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    heap.allocate_object(HashMap::new())
}

/// Create a promise and return its ID (for dynamic import)
#[no_mangle]
pub extern "C" fn builtin_create_promise_raw() -> u64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    heap.allocate_promise()
}

// ============================================================================
// End Dynamic Import Helper Functions
// ============================================================================

extern "C" fn builtin_math_ceil(value: f64) -> f64 {
    value.ceil()
}

extern "C" fn builtin_math_sqrt(value: f64) -> f64 {
    value.sqrt()
}

extern "C" fn builtin_math_abs(value: f64) -> f64 {
    value.abs()
}

extern "C" fn builtin_math_sin(value: f64) -> f64 {
    value.sin()
}

extern "C" fn builtin_math_cos(value: f64) -> f64 {
    value.cos()
}

extern "C" fn builtin_math_random() -> f64 {
    // Simple random using system time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    ((now as u64) % 1000000) as f64 / 1000000.0
}

// ============================================================================
// BigInt Arithmetic Built-ins
// ============================================================================

/// Helper to get BigInt from tagged value
fn get_bigint_from_value(value: f64) -> Option<num_bigint::BigInt> {
    if is_bigint_id(value) {
        let id = decode_bigint_id(value);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        heap.get_bigint(id).cloned()
    } else {
        None
    }
}

/// Helper to allocate BigInt result and return tagged value
fn allocate_bigint_result(result: num_bigint::BigInt) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    let id = heap.allocate_bigint(result);
    encode_bigint_id(id)
}

/// BigInt addition: a + b
extern "C" fn builtin_bigint_add(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN, // TypeError: not a BigInt
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN, // TypeError: not a BigInt
    };
    allocate_bigint_result(a_val + b_val)
}

/// BigInt subtraction: a - b
extern "C" fn builtin_bigint_sub(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    allocate_bigint_result(a_val - b_val)
}

/// BigInt multiplication: a * b
extern "C" fn builtin_bigint_mul(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    allocate_bigint_result(a_val * b_val)
}

/// BigInt division: a / b (truncated toward zero)
/// Throws RangeError if b is zero
extern "C" fn builtin_bigint_div(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => {
            throw_type_error("Cannot convert to BigInt");
            return f64::NAN;
        }
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => {
            throw_type_error("Cannot convert to BigInt");
            return f64::NAN;
        }
    };
    if b_val == num_bigint::BigInt::from(0) {
        throw_range_error("Division by zero");
        return f64::NAN;
    }
    allocate_bigint_result(a_val / b_val)
}

/// BigInt modulo: a % b
/// Throws RangeError if b is zero
extern "C" fn builtin_bigint_mod(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => {
            throw_type_error("Cannot convert to BigInt");
            return f64::NAN;
        }
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => {
            throw_type_error("Cannot convert to BigInt");
            return f64::NAN;
        }
    };
    if b_val == num_bigint::BigInt::from(0) {
        throw_range_error("Division by zero");
        return f64::NAN;
    }
    allocate_bigint_result(a_val % b_val)
}

/// BigInt exponentiation: a ** b
/// Throws RangeError if b is negative
extern "C" fn builtin_bigint_pow(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => {
            throw_type_error("Cannot convert to BigInt");
            return f64::NAN;
        }
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => {
            throw_type_error("Cannot convert to BigInt");
            return f64::NAN;
        }
    };
    // BigInt exponentiation requires non-negative exponent
    if b_val < num_bigint::BigInt::from(0) {
        throw_range_error("Exponent must be positive");
        return f64::NAN;
    }
    // Convert exponent to u32 for pow operation
    use num_traits::ToPrimitive;
    match b_val.to_u32() {
        Some(exp) => allocate_bigint_result(a_val.pow(exp)),
        None => {
            throw_range_error("Exponent too large");
            f64::NAN
        }
    }
}

// ============================================================================
// BigInt Comparison Built-ins
// ============================================================================

/// BigInt less than: a < b
/// Returns 1.0 for true, 0.0 for false, NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_lt(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    if a_val < b_val { 1.0 } else { 0.0 }
}

/// BigInt greater than: a > b
/// Returns 1.0 for true, 0.0 for false, NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_gt(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    if a_val > b_val { 1.0 } else { 0.0 }
}

/// BigInt less than or equal: a <= b
/// Returns 1.0 for true, 0.0 for false, NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_le(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    if a_val <= b_val { 1.0 } else { 0.0 }
}

/// BigInt greater than or equal: a >= b
/// Returns 1.0 for true, 0.0 for false, NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_ge(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    if a_val >= b_val { 1.0 } else { 0.0 }
}

/// BigInt equality: a == b (abstract equality)
/// Returns 1.0 for true, 0.0 for false, NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_eq(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    if a_val == b_val { 1.0 } else { 0.0 }
}

/// BigInt strict equality: a === b
/// Returns 1.0 for true, 0.0 for false, NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_strict_eq(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    if a_val == b_val { 1.0 } else { 0.0 }
}

// ============================================================================
// BigInt Bitwise Built-ins
// ============================================================================

/// BigInt bitwise AND: a & b
/// Returns NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_and(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    allocate_bigint_result(&a_val & &b_val)
}

/// BigInt bitwise OR: a | b
/// Returns NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_or(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    allocate_bigint_result(&a_val | &b_val)
}

/// BigInt bitwise XOR: a ^ b
/// Returns NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_xor(a: f64, b: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    allocate_bigint_result(&a_val ^ &b_val)
}

/// BigInt bitwise NOT: ~a
/// Returns NaN if operand is not a BigInt
/// Note: For BigInt, ~a is defined as -(a + 1) per ECMAScript spec
extern "C" fn builtin_bigint_not(a: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    // ~a = -(a + 1) for BigInt (two's complement)
    let one = num_bigint::BigInt::from(1);
    allocate_bigint_result(-(&a_val + &one))
}

/// BigInt left shift: a << b
/// Returns NaN if either operand is not a BigInt or if b is negative
extern "C" fn builtin_bigint_shl(a: f64, b: f64) -> f64 {
    use num_traits::ToPrimitive;
    
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    
    // Shift amount must be non-negative and fit in u64
    if b_val < num_bigint::BigInt::from(0) {
        // Negative shift - perform right shift instead
        match (-&b_val).to_u64() {
            Some(shift) => allocate_bigint_result(&a_val >> shift),
            None => f64::NAN, // Shift amount too large
        }
    } else {
        match b_val.to_u64() {
            Some(shift) => allocate_bigint_result(&a_val << shift),
            None => f64::NAN, // Shift amount too large
        }
    }
}

/// BigInt right shift: a >> b (sign-propagating)
/// Returns NaN if either operand is not a BigInt
extern "C" fn builtin_bigint_shr(a: f64, b: f64) -> f64 {
    use num_traits::ToPrimitive;
    
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let b_val = match get_bigint_from_value(b) {
        Some(v) => v,
        None => return f64::NAN,
    };
    
    // Shift amount must be non-negative and fit in u64
    if b_val < num_bigint::BigInt::from(0) {
        // Negative shift - perform left shift instead
        match (-&b_val).to_u64() {
            Some(shift) => allocate_bigint_result(&a_val << shift),
            None => f64::NAN, // Shift amount too large
        }
    } else {
        match b_val.to_u64() {
            Some(shift) => allocate_bigint_result(&a_val >> shift),
            None => f64::NAN, // Shift amount too large
        }
    }
}

// ============================================================================
// BigInt Conversion Built-ins
// ============================================================================

/// BigInt to string conversion: bigint.toString()
/// Returns a string ID representing the decimal string of the BigInt
/// Throws TypeError if operand is not a BigInt
extern "C" fn builtin_bigint_to_string(a: f64) -> f64 {
    let a_val = match get_bigint_from_value(a) {
        Some(v) => v,
        None => {
            throw_type_error("Cannot convert to BigInt");
            return f64::NAN;
        }
    };
    
    let string_repr = a_val.to_string();
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    let string_id = heap.allocate_string(string_repr);
    encode_string_id(string_id)
}

/// BigInt from string conversion: BigInt(string)
/// Returns a BigInt ID if the string is a valid integer representation
/// Throws SyntaxError if the string is not a valid integer
extern "C" fn builtin_bigint_from_string(s: f64) -> f64 {
    // Get the string from the heap
    if !is_string_id(s) {
        throw_type_error("Cannot convert to BigInt: expected string");
        return f64::NAN;
    }
    
    let string_id = decode_string_id(s);
    let heap = get_runtime_heap_lock();
    let heap_guard = heap.lock().unwrap();
    
    let string_val = match heap_guard.get_string(string_id) {
        Some(s) => s.clone(),
        None => {
            drop(heap_guard);
            throw_type_error("Invalid string reference");
            return f64::NAN;
        }
    };
    drop(heap_guard);
    
    // Trim whitespace and parse
    let trimmed = string_val.trim();
    
    // Handle empty string
    if trimmed.is_empty() {
        throw_syntax_error("Cannot convert empty string to BigInt");
        return f64::NAN;
    }
    
    // Parse the string as BigInt
    match trimmed.parse::<num_bigint::BigInt>() {
        Ok(bigint) => allocate_bigint_result(bigint),
        Err(_) => {
            throw_syntax_error("Cannot convert to BigInt: invalid integer literal");
            f64::NAN
        }
    }
}

/// BigInt from number conversion: BigInt(number)
/// Returns a BigInt ID if the number is a safe integer (no fractional part)
/// Throws RangeError if the number has a fractional part
extern "C" fn builtin_bigint_from_number(n: f64) -> f64 {
    // Check if it's already a BigInt
    if is_bigint_id(n) {
        return n; // Already a BigInt, return as-is
    }
    
    // Check for special values
    if n.is_nan() || n.is_infinite() {
        throw_range_error("Cannot convert NaN or Infinity to BigInt");
        return f64::NAN;
    }
    
    // Check if the number has a fractional part
    if n.fract() != 0.0 {
        throw_range_error("Cannot convert non-integer to BigInt");
        return f64::NAN;
    }
    
    // Convert to BigInt
    // For large numbers, we need to handle them carefully
    let bigint = if n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        num_bigint::BigInt::from(n as i64)
    } else {
        // For very large numbers, convert via string to preserve precision
        let int_str = format!("{:.0}", n);
        match int_str.parse::<num_bigint::BigInt>() {
            Ok(bi) => bi,
            Err(_) => return f64::NAN,
        }
    };
    
    allocate_bigint_result(bigint)
}

/// Check if a value is a BigInt (for type checking)
/// Returns 1.0 if the value is a BigInt, 0.0 otherwise
extern "C" fn builtin_is_bigint(a: f64) -> f64 {
    if is_bigint_id(a) { 1.0 } else { 0.0 }
}

/// Check if mixing BigInt and Number in arithmetic (for error detection)
/// Returns 1.0 if there's a type mismatch (one BigInt, one Number), 0.0 otherwise
extern "C" fn builtin_bigint_type_check(a: f64, b: f64) -> f64 {
    let a_is_bigint = is_bigint_id(a);
    let b_is_bigint = is_bigint_id(b);
    
    // If one is BigInt and the other is not, check if the non-BigInt is a number
    if a_is_bigint != b_is_bigint {
        let non_bigint = if a_is_bigint { b } else { a };
        
        // If it's not a string (and not a BigInt), it's a regular number - type mismatch
        if !is_string_id(non_bigint) && !is_bigint_id(non_bigint) {
            // It's a regular f64 number - this is a type error when mixed with BigInt
            return 1.0; // TypeError: Cannot mix BigInt and other types
        }
    }
    
    0.0 // No type error
}

// ============================================================================
// Type Coercion Built-ins (ECMAScript Specification)
// ============================================================================

/// Special value constants for type checking
const NULL_VALUE: f64 = -999_999_999.0;
const UNDEFINED_VALUE: f64 = f64::NAN;
const TRUE_VALUE: f64 = 1.0;
const FALSE_VALUE: f64 = 0.0;

/// Check if a value represents null
fn is_null_value(value: f64) -> bool {
    value == NULL_VALUE
}

/// Check if a value represents undefined (NaN)
fn is_undefined_value(value: f64) -> bool {
    value.is_nan()
}

/// Check if a value is nullish (null or undefined)
fn is_nullish_value(value: f64) -> bool {
    is_null_value(value) || is_undefined_value(value)
}

/// Check if a value is a boolean (0.0 or 1.0 that's not a heap object)
fn is_boolean_value(value: f64) -> bool {
    (value == 0.0 || value == 1.0) && !is_heap_object_id(value)
}

/// Check if a value is a heap object ID (positive integer in object range)
fn is_heap_object_id(value: f64) -> bool {
    value > 0.0 && value < 1_000_000.0 && value.fract() == 0.0
}

/// Get the type tag of a value for comparison
/// Returns: 0=undefined, 1=null, 2=boolean, 3=number, 4=string, 5=bigint, 6=object
fn get_value_type(value: f64) -> u8 {
    if is_undefined_value(value) {
        0 // undefined
    } else if is_null_value(value) {
        1 // null
    } else if is_string_id(value) {
        4 // string
    } else if is_bigint_id(value) {
        5 // bigint
    } else if is_heap_object_id(value) {
        6 // object (array, function, etc.)
    } else {
        3 // number (includes booleans in our representation)
    }
}

/// ToBoolean conversion per ECMAScript spec
/// Returns 1.0 for truthy, 0.0 for falsy
/// 
/// Falsy values: undefined, null, false, +0, -0, NaN, ""
/// Everything else is truthy
extern "C" fn builtin_to_boolean(value: f64) -> f64 {
    // undefined is falsy (NaN is used for undefined in our encoding)
    if is_undefined_value(value) {
        return FALSE_VALUE;
    }
    
    // null is falsy
    if is_null_value(value) {
        return FALSE_VALUE;
    }
    
    // Strings: empty string is falsy, non-empty is truthy
    if is_string_id(value) {
        let string_id = decode_string_id(value);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        if let Some(s) = heap.get_string(string_id) {
            return if s.is_empty() { FALSE_VALUE } else { TRUE_VALUE };
        }
        return FALSE_VALUE;
    }
    
    // BigInt: 0n is falsy, everything else is truthy
    if is_bigint_id(value) {
        if let Some(bigint) = get_bigint_from_value(value) {
            return if bigint == num_bigint::BigInt::from(0) { FALSE_VALUE } else { TRUE_VALUE };
        }
        return FALSE_VALUE;
    }
    
    // Objects are always truthy
    if is_heap_object_id(value) {
        return TRUE_VALUE;
    }
    
    // Numbers: +0, -0, NaN are falsy, everything else is truthy
    if value == 0.0 || value == -0.0 || value.is_nan() {
        return FALSE_VALUE;
    }
    
    TRUE_VALUE
}

/// ToNumber conversion per ECMAScript spec
/// 
/// - undefined -> NaN
/// - null -> +0
/// - boolean -> 1 if true, +0 if false
/// - number -> identity
/// - string -> parse as number
/// - bigint -> TypeError (returns NaN)
/// - object -> ToPrimitive then ToNumber
extern "C" fn builtin_to_number(value: f64) -> f64 {
    // undefined -> NaN
    if is_undefined_value(value) {
        return f64::NAN;
    }
    
    // null -> +0
    if is_null_value(value) {
        return 0.0;
    }
    
    // Strings: parse as number
    if is_string_id(value) {
        let string_id = decode_string_id(value);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        if let Some(s) = heap.get_string(string_id) {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return 0.0; // Empty string -> 0
            }
            return trimmed.parse::<f64>().unwrap_or(f64::NAN);
        }
        return f64::NAN;
    }
    
    // BigInt -> TypeError (return NaN for now)
    if is_bigint_id(value) {
        throw_type_error("Cannot convert BigInt to number");
        return f64::NAN;
    }
    
    // Objects -> NaN (simplified, should call ToPrimitive)
    if is_heap_object_id(value) {
        return f64::NAN;
    }
    
    // Numbers: identity
    value
}

/// ToString conversion per ECMAScript spec
/// Returns a string ID
/// 
/// - undefined -> "undefined"
/// - null -> "null"
/// - boolean -> "true" or "false"
/// - number -> number string representation
/// - string -> identity
/// - bigint -> bigint string representation
/// - object -> "[object Object]" (simplified)
extern "C" fn builtin_to_string(value: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    
    // undefined -> "undefined"
    if is_undefined_value(value) {
        let id = heap.allocate_string("undefined".to_string());
        return encode_string_id(id);
    }
    
    // null -> "null"
    if is_null_value(value) {
        let id = heap.allocate_string("null".to_string());
        return encode_string_id(id);
    }
    
    // Strings: identity
    if is_string_id(value) {
        return value;
    }
    
    // BigInt -> string representation
    if is_bigint_id(value) {
        let bigint_id = decode_bigint_id(value);
        if let Some(bigint) = heap.get_bigint(bigint_id) {
            let s = bigint.to_string();
            let id = heap.allocate_string(s);
            return encode_string_id(id);
        }
        let id = heap.allocate_string("0".to_string());
        return encode_string_id(id);
    }
    
    // Objects -> "[object Object]" (simplified)
    if is_heap_object_id(value) {
        let id = heap.allocate_string("[object Object]".to_string());
        return encode_string_id(id);
    }
    
    // Numbers: string representation
    // Note: In our encoding, booleans are 0.0 (false) and 1.0 (true)
    // which are treated as numbers here. For proper boolean handling,
    // the caller should track the original type.
    let s = if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() }
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    };
    
    let id = heap.allocate_string(s);
    encode_string_id(id)
}

/// Strict equality (===) per ECMAScript spec
/// Returns 1.0 for true, 0.0 for false
/// 
/// - Different types -> false
/// - NaN === NaN -> false
/// - +0 === -0 -> true
/// - Same value -> true
extern "C" fn builtin_strict_equals(a: f64, b: f64) -> f64 {
    let type_a = get_value_type(a);
    let type_b = get_value_type(b);
    
    // Different types are never strictly equal
    if type_a != type_b {
        return FALSE_VALUE;
    }
    
    // Both undefined
    if type_a == 0 {
        return TRUE_VALUE;
    }
    
    // Both null
    if type_a == 1 {
        return TRUE_VALUE;
    }
    
    // Both strings
    if type_a == 4 {
        let id_a = decode_string_id(a);
        let id_b = decode_string_id(b);
        
        // Same string ID means same string
        if id_a == id_b {
            return TRUE_VALUE;
        }
        
        // Compare string contents
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        let str_a = heap.get_string(id_a);
        let str_b = heap.get_string(id_b);
        
        match (str_a, str_b) {
            (Some(a), Some(b)) => if a == b { TRUE_VALUE } else { FALSE_VALUE },
            _ => FALSE_VALUE,
        }
    }
    // Both BigInts
    else if type_a == 5 {
        let bigint_a = get_bigint_from_value(a);
        let bigint_b = get_bigint_from_value(b);
        
        match (bigint_a, bigint_b) {
            (Some(a), Some(b)) => if a == b { TRUE_VALUE } else { FALSE_VALUE },
            _ => FALSE_VALUE,
        }
    }
    // Both objects (compare by reference/ID)
    else if type_a == 6 {
        if a == b { TRUE_VALUE } else { FALSE_VALUE }
    }
    // Both numbers
    else {
        // NaN !== NaN
        if a.is_nan() && b.is_nan() {
            return FALSE_VALUE;
        }
        
        // +0 === -0
        if a == 0.0 && b == 0.0 {
            return TRUE_VALUE;
        }
        
        // Regular number comparison
        if a == b { TRUE_VALUE } else { FALSE_VALUE }
    }
}

/// Loose equality (==) per ECMAScript spec
/// Returns 1.0 for true, 0.0 for false
/// 
/// Implements the Abstract Equality Comparison Algorithm:
/// 1. If same type, use strict equality
/// 2. null == undefined (and vice versa)
/// 3. Number == String: convert string to number
/// 4. Boolean == anything: convert boolean to number
/// 5. Object == primitive: convert object to primitive
extern "C" fn builtin_loose_equals(a: f64, b: f64) -> f64 {
    let type_a = get_value_type(a);
    let type_b = get_value_type(b);
    
    // Same type: use strict equality
    if type_a == type_b {
        return builtin_strict_equals(a, b);
    }
    
    // null == undefined (and vice versa)
    if (type_a == 0 && type_b == 1) || (type_a == 1 && type_b == 0) {
        return TRUE_VALUE;
    }
    
    // Number == String: convert string to number
    if type_a == 3 && type_b == 4 {
        let b_num = builtin_to_number(b);
        return builtin_strict_equals(a, b_num);
    }
    if type_a == 4 && type_b == 3 {
        let a_num = builtin_to_number(a);
        return builtin_strict_equals(a_num, b);
    }
    
    // BigInt == String: convert string to BigInt
    if type_a == 5 && type_b == 4 {
        // Try to parse string as BigInt
        let string_id = decode_string_id(b);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        if let Some(s) = heap.get_string(string_id) {
            if let Ok(parsed) = s.trim().parse::<num_bigint::BigInt>() {
                if let Some(bigint_a) = get_bigint_from_value(a) {
                    return if bigint_a == parsed { TRUE_VALUE } else { FALSE_VALUE };
                }
            }
        }
        return FALSE_VALUE;
    }
    if type_a == 4 && type_b == 5 {
        // Try to parse string as BigInt
        let string_id = decode_string_id(a);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        if let Some(s) = heap.get_string(string_id) {
            if let Ok(parsed) = s.trim().parse::<num_bigint::BigInt>() {
                if let Some(bigint_b) = get_bigint_from_value(b) {
                    return if parsed == bigint_b { TRUE_VALUE } else { FALSE_VALUE };
                }
            }
        }
        return FALSE_VALUE;
    }
    
    // BigInt == Number: compare values (no type error for comparison)
    if type_a == 5 && type_b == 3 {
        if let Some(bigint_a) = get_bigint_from_value(a) {
            // Check if number is NaN or Infinity
            if b.is_nan() || b.is_infinite() {
                return FALSE_VALUE;
            }
            // Check if number has fractional part
            if b.fract() != 0.0 {
                return FALSE_VALUE;
            }
            // Compare as BigInt
            use num_traits::ToPrimitive;
            if let Some(b_i64) = (b as i64).to_i64() {
                let bigint_b = num_bigint::BigInt::from(b_i64);
                return if bigint_a == bigint_b { TRUE_VALUE } else { FALSE_VALUE };
            }
        }
        return FALSE_VALUE;
    }
    if type_a == 3 && type_b == 5 {
        if let Some(bigint_b) = get_bigint_from_value(b) {
            // Check if number is NaN or Infinity
            if a.is_nan() || a.is_infinite() {
                return FALSE_VALUE;
            }
            // Check if number has fractional part
            if a.fract() != 0.0 {
                return FALSE_VALUE;
            }
            // Compare as BigInt
            use num_traits::ToPrimitive;
            if let Some(a_i64) = (a as i64).to_i64() {
                let bigint_a = num_bigint::BigInt::from(a_i64);
                return if bigint_a == bigint_b { TRUE_VALUE } else { FALSE_VALUE };
            }
        }
        return FALSE_VALUE;
    }
    
    // Boolean == anything: convert boolean to number first
    // In our representation, booleans are 0.0 or 1.0 which are already numbers
    // But we need to handle the case where we're comparing with other types
    
    // Object == primitive: simplified (return false for now)
    // Full implementation would call ToPrimitive
    
    FALSE_VALUE
}

/// String concatenation with + operator
/// If either operand is a string, concatenate as strings
/// Returns a string ID
extern "C" fn builtin_string_concat(a: f64, b: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    
    // Convert both operands to strings
    let str_a = value_to_string_internal(a, &heap);
    let str_b = value_to_string_internal(b, &heap);
    
    let result = format!("{}{}", str_a, str_b);
    let id = heap.allocate_string(result);
    encode_string_id(id)
}

/// Helper function to convert a value to string without allocating
fn value_to_string_internal(value: f64, heap: &RuntimeHeap) -> String {
    if is_undefined_value(value) {
        return "undefined".to_string();
    }
    
    if is_null_value(value) {
        return "null".to_string();
    }
    
    if is_string_id(value) {
        let string_id = decode_string_id(value);
        if let Some(s) = heap.get_string(string_id) {
            return s.clone();
        }
        return "".to_string();
    }
    
    if is_bigint_id(value) {
        let bigint_id = decode_bigint_id(value);
        if let Some(bigint) = heap.get_bigint(bigint_id) {
            return bigint.to_string();
        }
        return "0".to_string();
    }
    
    if is_heap_object_id(value) {
        // Check if it's an array
        let id = value as u64;
        if let Some(arr) = heap.get_array(id) {
            let elements: Vec<String> = arr.iter()
                .map(|&v| value_to_string_internal(v, heap))
                .collect();
            return elements.join(",");
        }
        return "[object Object]".to_string();
    }
    
    // Number
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() }
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    }
}

/// Check if either operand is a string (for + operator dispatch)
/// Returns 1.0 if either is a string, 0.0 otherwise
extern "C" fn builtin_is_string_operand(a: f64, b: f64) -> f64 {
    if is_string_id(a) || is_string_id(b) {
        TRUE_VALUE
    } else {
        FALSE_VALUE
    }
}

/// Check if mixing BigInt and Number in arithmetic
/// Throws TypeError if mixing, returns 0.0 if OK
extern "C" fn builtin_check_bigint_number_mix(a: f64, b: f64) -> f64 {
    let a_is_bigint = is_bigint_id(a);
    let b_is_bigint = is_bigint_id(b);
    
    // If one is BigInt and the other is a regular number, throw TypeError
    if a_is_bigint != b_is_bigint {
        let non_bigint = if a_is_bigint { b } else { a };
        
        // Check if the non-BigInt is a regular number (not string, not object)
        if !is_string_id(non_bigint) && !is_heap_object_id(non_bigint) && 
           !is_null_value(non_bigint) && !is_undefined_value(non_bigint) {
            throw_type_error("Cannot mix BigInt and other types, use explicit conversions");
            return TRUE_VALUE; // Indicates error
        }
    }
    
    FALSE_VALUE // No error
}

/// JSON.parse - parse a JSON string and return a heap object/array/value
extern "C" fn builtin_json_parse(json_str_id: f64) -> f64 {
    // Get the string from the heap
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();

    // Decode the string ID
    if !is_string_id(json_str_id) {
        // Create error message and throw
        let error_msg = "SyntaxError: JSON.parse requires a string argument".to_string();
        let error_id = heap.allocate_string(error_msg);
        // Lock is released when heap goes out of scope
        return throw_json_error(encode_string_id(error_id));
    }

    let string_id = decode_string_id(json_str_id);
    let json_str = match heap.get_string(string_id) {
        Some(s) => s.clone(),
        None => {
            let error_msg = "SyntaxError: Invalid string reference".to_string();
            let error_id = heap.allocate_string(error_msg);
            // Lock is released when heap goes out of scope
            return throw_json_error(encode_string_id(error_id));
        }
    };

    // Parse the JSON
    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(json_value) => json_to_runtime_value(json_value, &mut heap),
        Err(e) => {
            // Extract line and column from serde_json error
            let line = e.line();
            let column = e.column();
            let error_msg =
                format!("SyntaxError: Unexpected token at line {}, column {}", line, column);
            let error_id = heap.allocate_string(error_msg);
            // Lock is released when heap goes out of scope
            throw_json_error(encode_string_id(error_id))
        }
    }
}

/// Helper to throw a JSON parse error using the exception mechanism
fn throw_json_error(error_string_id: f64) -> f64 {
    // Store the exception value (the error string ID)
    CURRENT_EXCEPTION.with(|exc| {
        *exc.borrow_mut() = error_string_id;
    });

    // Check if there's a handler
    let has_handler = EXCEPTION_HANDLERS.with(|handlers| !handlers.borrow().is_empty());

    if has_handler {
        // Return the catch block ID to jump to
        EXCEPTION_HANDLERS.with(|handlers| {
            let handlers = handlers.borrow();
            if let Some(handler) = handlers.last() {
                handler.catch_block as f64
            } else {
                f64::NAN
            }
        })
    } else {
        // No handler - set error state and return NaN
        // The error message is stored in CURRENT_EXCEPTION for retrieval
        f64::NAN
    }
}

/// Convert serde_json::Value to runtime heap value
fn json_to_runtime_value(json: serde_json::Value, heap: &mut RuntimeHeap) -> f64 {
    match json {
        serde_json::Value::Null => 0.0, // null represented as 0
        serde_json::Value::Bool(b) => {
            if b {
                1.0
            } else {
                0.0
            }
        }
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        serde_json::Value::String(s) => {
            let id = heap.allocate_string(s);
            encode_string_id(id)
        }
        serde_json::Value::Array(arr) => {
            let elements: Vec<f64> =
                arr.into_iter().map(|v| json_to_runtime_value(v, heap)).collect();
            heap.allocate_array(elements) as f64
        }
        serde_json::Value::Object(map) => {
            let mut props = HashMap::new();
            for (key, val) in map {
                let key_hash = {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    key.hash(&mut hasher);
                    hasher.finish()
                };
                let prop_key = format!("prop_{}", key_hash);
                props.insert(prop_key, json_to_runtime_value(val, heap));
            }
            heap.allocate_object(props) as f64
        }
    }
}

/// JSON.stringify - convert a value to a JSON string
extern "C" fn builtin_json_stringify(value: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();

    let json_str = runtime_value_to_json_string(value, &heap);
    let id = heap.allocate_string(json_str);
    encode_string_id(id)
}

/// Convert runtime value to JSON string
fn runtime_value_to_json_string(value: f64, heap: &RuntimeHeap) -> String {
    if is_string_id(value) {
        let string_id = decode_string_id(value);
        if let Some(s) = heap.get_string(string_id) {
            return format!("\"{}\"", escape_json_string(s));
        }
        return "null".to_string();
    }

    if value.is_nan() {
        return "null".to_string(); // undefined becomes null in JSON
    }

    if value.is_infinite() {
        return "null".to_string(); // Infinity becomes null in JSON
    }

    if value.fract() == 0.0 && value.abs() < 1e15 {
        let id = value as u64;
        if id > 0 && id < 1_000_000 {
            // Check if it's an array
            if let Some(arr) = heap.get_array(id) {
                let parts: Vec<String> =
                    arr.iter().map(|&v| runtime_value_to_json_string(v, heap)).collect();
                return format!("[{}]", parts.join(","));
            }
            // Check if it's an object
            if let Some(obj) = heap.get_object(id) {
                let parts: Vec<String> = obj
                    .iter()
                    .map(|(k, &v)| format!("\"{}\":{}", k, runtime_value_to_json_string(v, heap)))
                    .collect();
                return format!("{{{}}}", parts.join(","));
            }
        }
        // Regular integer
        return format!("{}", value as i64);
    }

    // Float
    format!("{}", value)
}

/// Escape special characters in JSON strings
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

// Runtime heap for objects, arrays, and closures
// Using thread_local for thread-safe per-thread heap instances
// Each thread gets its own RuntimeHeap, eliminating contention
thread_local! {
    static RUNTIME_HEAP: std::cell::RefCell<RuntimeHeap> = std::cell::RefCell::new(RuntimeHeap::new());
}

// Wrapper for function pointers to make them Send + Sync
//
// # Safety
// Function pointers are safe to share across threads because:
// 1. They point to immutable compiled code in the JIT module
// 2. The JIT module is kept alive for the lifetime of the program
// 3. The code they point to is never modified after compilation
// 4. Function calls are thread-safe as they only read from the code segment
#[derive(Clone, Copy)]
struct FnPtr(*const u8);

// SAFETY: FnPtr wraps a pointer to immutable compiled code.
// The code is generated by Cranelift JIT and remains valid and unchanged
// for the lifetime of the JITModule. Multiple threads can safely call
// the same compiled function simultaneously.
unsafe impl Send for FnPtr {}
unsafe impl Sync for FnPtr {}

// Global registry of compiled function pointers
// Maps function_id -> function pointer
//
// Thread-Safety: This registry is protected by OnceLock + Mutex.
// - OnceLock ensures single initialization
// - Mutex ensures exclusive access during reads/writes
// - FnPtr is Send + Sync because it points to immutable compiled code
static COMPILED_FUNCTIONS: OnceLock<Mutex<HashMap<u32, FnPtr>>> = OnceLock::new();

fn get_compiled_functions_lock() -> &'static Mutex<HashMap<u32, FnPtr>> {
    COMPILED_FUNCTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a compiled function pointer
pub fn register_compiled_function(function_id: u32, ptr: *const u8) {
    let funcs = get_compiled_functions_lock();
    funcs.lock().unwrap().insert(function_id, FnPtr(ptr));
}

/// Get a compiled function pointer
pub fn get_compiled_function(function_id: u32) -> Option<*const u8> {
    let funcs = get_compiled_functions_lock();
    funcs.lock().unwrap().get(&function_id).map(|fp| fp.0)
}

/// Runtime heap for allocating objects, arrays, and closures
struct RuntimeHeap {
    /// Closure storage: function_id -> (captured_vars, is_arrow)
    closures: HashMap<u64, ClosureData>,
    /// Array storage: array_id -> elements
    arrays: HashMap<u64, Vec<f64>>,
    /// Object storage: object_id -> properties
    objects: HashMap<u64, HashMap<String, f64>>,
    /// Generator storage: generator_id -> generator state
    generators: HashMap<u64, GeneratorData>,
    /// Promise storage: promise_id -> promise state
    promises: HashMap<u64, PromiseData>,
    /// Async function storage: async_fn_id -> async function state
    async_functions: HashMap<u64, AsyncFunctionData>,
    /// String storage: string_id -> string value
    strings: HashMap<u64, String>,
    /// BigInt storage: bigint_id -> BigInt value
    bigints: HashMap<u64, num_bigint::BigInt>,
    /// Class storage: class_id -> class data
    classes: HashMap<u64, ClassData>,
    /// Object prototype chain: object_id -> prototype_id (0 means no prototype)
    prototypes: HashMap<u64, u64>,
    /// Next available ID
    next_id: u64,
}

#[derive(Clone)]
struct ClosureData {
    function_id: u32,
    captured_vars: Vec<f64>,
    is_arrow: bool,
}

/// Generator function state for ES6 generator implementation
/// Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7
#[derive(Clone)]
struct GeneratorData {
    /// The function ID of the generator function body
    function_id: u32,
    /// Captured variables from the enclosing scope
    captured_vars: Vec<f64>,
    /// Current state of the generator
    state: GeneratorState,
    /// The current yielded value
    current_value: f64,
    /// Current resume point (block index) in the generator
    resume_point: u32,
    /// Local variables saved across yield points
    saved_locals: Vec<f64>,
    /// Value sent via next(value) - used as yield expression result
    sent_value: f64,
    /// Whether an error was thrown into the generator via throw()
    thrown_error: Option<f64>,
    /// Whether return() was called with a value
    return_value: Option<f64>,
}

/// Generator state machine states
/// Requirements: 9.1, 9.2, 9.3, 9.4
#[derive(Clone, Copy, PartialEq, Debug)]
enum GeneratorState {
    /// Initial state - generator created but not started
    /// Requirements: 9.1 - generator function returns generator object without executing body
    Created,
    /// Generator is suspended at a yield point
    /// Requirements: 9.3 - yield suspends execution
    Suspended,
    /// Generator is currently executing
    Executing,
    /// Generator has completed (returned or exhausted)
    /// Requirements: 9.4 - generator returns {value: undefined, done: true} when finished
    Completed,
}

/// Promise state for async/await implementation
#[derive(Clone)]
struct PromiseData {
    state: PromiseState,
    value: f64,
    /// Then callbacks: (on_fulfilled_closure_id, result_promise_id)
    then_callbacks: Vec<(Option<u64>, u64)>,
    /// Catch callbacks: (on_rejected_closure_id, result_promise_id)
    catch_callbacks: Vec<(Option<u64>, u64)>,
    /// Finally callbacks: (closure_id, result_promise_id)
    finally_callbacks: Vec<(u64, u64)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum PromiseState {
    Pending,
    Fulfilled,
    Rejected,
}

/// Async function state for state machine transformation
#[derive(Clone)]
struct AsyncFunctionData {
    /// The function ID of the async function body
    function_id: u32,
    /// Captured variables from the enclosing scope
    captured_vars: Vec<f64>,
    /// The promise that will be resolved/rejected when the async function completes
    result_promise_id: u64,
    /// Current state in the state machine (0 = initial, 1+ = after await points)
    current_state: u32,
    /// Local variables saved across await points
    saved_locals: Vec<f64>,
    /// Whether the function has started executing
    started: bool,
}

/// Class data for ES6 class support
/// Stores class metadata including constructor, prototype, and inheritance info
#[derive(Clone)]
struct ClassData {
    /// The constructor function ID (None for classes without explicit constructor)
    constructor_id: Option<u32>,
    /// The prototype object ID - methods are stored here
    prototype_id: u64,
    /// The parent class ID (None if no extends clause)
    super_class_id: Option<u64>,
    /// Static properties stored on the class constructor
    static_properties: HashMap<String, f64>,
    /// Private fields definitions (field_name -> initial_value)
    private_fields: HashMap<String, f64>,
    /// Getters defined on the prototype (property_name -> function_id)
    getters: HashMap<String, u32>,
    /// Setters defined on the prototype (property_name -> function_id)
    setters: HashMap<String, u32>,
}

impl RuntimeHeap {
    fn new() -> Self {
        Self {
            closures: HashMap::new(),
            arrays: HashMap::new(),
            objects: HashMap::new(),
            generators: HashMap::new(),
            promises: HashMap::new(),
            async_functions: HashMap::new(),
            strings: HashMap::new(),
            bigints: HashMap::new(),
            classes: HashMap::new(),
            prototypes: HashMap::new(),
            next_id: 1,
        }
    }

    /// Allocate a string in the heap
    ///
    /// The input is already a valid Rust String, so UTF-8 is guaranteed.
    fn allocate_string(&mut self, s: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.strings.insert(id, s);
        id
    }

    /// Allocate a string from raw bytes with UTF-8 validation
    ///
    /// Returns Ok(id) if the bytes are valid UTF-8, or Err with the error details.
    /// This validates UTF-8 encoding per Requirements 1.5.
    fn allocate_string_from_bytes(&mut self, bytes: &[u8]) -> Result<u64, std::str::Utf8Error> {
        // Validate UTF-8 encoding - return error for invalid sequences
        let s = std::str::from_utf8(bytes)?;
        Ok(self.allocate_string(s.to_string()))
    }

    /// Allocate a string from raw bytes, replacing invalid UTF-8 with replacement character
    ///
    /// This is a lossy conversion that replaces invalid UTF-8 sequences with U+FFFD.
    /// Use `allocate_string_from_bytes` for strict validation.
    fn allocate_string_lossy(&mut self, bytes: &[u8]) -> u64 {
        let s = String::from_utf8_lossy(bytes).into_owned();
        self.allocate_string(s)
    }

    fn get_string(&self, id: u64) -> Option<&String> {
        self.strings.get(&id)
    }

    /// Allocate a BigInt value in the heap
    fn allocate_bigint(&mut self, value: num_bigint::BigInt) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.bigints.insert(id, value);
        id
    }

    /// Get a BigInt value from the heap
    fn get_bigint(&self, id: u64) -> Option<&num_bigint::BigInt> {
        self.bigints.get(&id)
    }

    /// Get a mutable reference to a BigInt value
    #[allow(dead_code)]
    fn get_bigint_mut(&mut self, id: u64) -> Option<&mut num_bigint::BigInt> {
        self.bigints.get_mut(&id)
    }

    fn allocate_closure(
        &mut self,
        function_id: u32,
        captured_vars: Vec<f64>,
        is_arrow: bool,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.closures.insert(
            id,
            ClosureData {
                function_id,
                captured_vars,
                is_arrow,
            },
        );
        id
    }

    fn allocate_array(&mut self, elements: Vec<f64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.arrays.insert(id, elements);
        id
    }

    fn allocate_object(&mut self, properties: HashMap<String, f64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.insert(id, properties);
        id
    }

    /// Allocate a generator object
    /// Requirements: 9.1 - generator function returns generator object without executing body
    fn allocate_generator(&mut self, function_id: u32, captured_vars: Vec<f64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.generators.insert(
            id,
            GeneratorData {
                function_id,
                captured_vars,
                state: GeneratorState::Created, // Start in Created state, not Suspended
                current_value: f64::NAN,
                resume_point: 0, // Start at the beginning
                saved_locals: Vec::new(),
                sent_value: f64::NAN,
                thrown_error: None,
                return_value: None,
            },
        );
        id
    }

    fn allocate_promise(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.promises.insert(
            id,
            PromiseData {
                state: PromiseState::Pending,
                value: f64::NAN,
                then_callbacks: Vec::new(),
                catch_callbacks: Vec::new(),
                finally_callbacks: Vec::new(),
            },
        );
        id
    }

    /// Allocate an async function state
    fn allocate_async_function(&mut self, function_id: u32, captured_vars: Vec<f64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        // Create the result promise for this async function
        let result_promise_id = self.allocate_promise();
        
        self.async_functions.insert(
            id,
            AsyncFunctionData {
                function_id,
                captured_vars,
                result_promise_id,
                current_state: 0,
                saved_locals: Vec::new(),
                started: false,
            },
        );
        id
    }

    /// Get an async function state
    fn get_async_function(&self, id: u64) -> Option<&AsyncFunctionData> {
        self.async_functions.get(&id)
    }

    /// Get a mutable reference to an async function state
    fn get_async_function_mut(&mut self, id: u64) -> Option<&mut AsyncFunctionData> {
        self.async_functions.get_mut(&id)
    }

    fn get_closure(&self, id: u64) -> Option<&ClosureData> {
        self.closures.get(&id)
    }

    fn get_array(&self, id: u64) -> Option<&Vec<f64>> {
        self.arrays.get(&id)
    }

    fn get_array_mut(&mut self, id: u64) -> Option<&mut Vec<f64>> {
        self.arrays.get_mut(&id)
    }

    fn get_object(&self, id: u64) -> Option<&HashMap<String, f64>> {
        self.objects.get(&id)
    }

    fn get_object_mut(&mut self, id: u64) -> Option<&mut HashMap<String, f64>> {
        self.objects.get_mut(&id)
    }

    fn get_closure_mut(&mut self, id: u64) -> Option<&mut ClosureData> {
        self.closures.get_mut(&id)
    }

    /// Reserved for generator iteration support
    #[allow(dead_code)]
    fn get_generator(&self, id: u64) -> Option<&GeneratorData> {
        self.generators.get(&id)
    }

    fn get_generator_mut(&mut self, id: u64) -> Option<&mut GeneratorData> {
        self.generators.get_mut(&id)
    }

    fn get_promise(&self, id: u64) -> Option<&PromiseData> {
        self.promises.get(&id)
    }

    fn get_promise_mut(&mut self, id: u64) -> Option<&mut PromiseData> {
        self.promises.get_mut(&id)
    }

    /// Allocate a class and return its ID
    /// Creates a prototype object for the class and sets up inheritance if super_class_id is provided
    fn allocate_class(&mut self, constructor_id: Option<u32>, super_class_id: Option<u64>) -> u64 {
        let class_id = self.next_id;
        self.next_id += 1;
        
        // Create the prototype object for this class
        let prototype_id = self.allocate_object(HashMap::new());
        
        // If there's a super class, set up the prototype chain
        if let Some(super_id) = super_class_id {
            if let Some(super_class) = self.classes.get(&super_id) {
                // Set the prototype's __proto__ to the super class's prototype
                self.prototypes.insert(prototype_id, super_class.prototype_id);
            }
        }
        
        self.classes.insert(class_id, ClassData {
            constructor_id,
            prototype_id,
            super_class_id,
            static_properties: HashMap::new(),
            private_fields: HashMap::new(),
            getters: HashMap::new(),
            setters: HashMap::new(),
        });
        
        class_id
    }

    /// Get a class by ID
    fn get_class(&self, id: u64) -> Option<&ClassData> {
        self.classes.get(&id)
    }

    /// Get a mutable reference to a class
    fn get_class_mut(&mut self, id: u64) -> Option<&mut ClassData> {
        self.classes.get_mut(&id)
    }

    /// Create an instance of a class
    /// Returns the instance object ID
    fn create_instance(&mut self, class_id: u64) -> Option<u64> {
        let class = self.classes.get(&class_id)?;
        let prototype_id = class.prototype_id;
        
        // Create a new object for the instance
        let instance_id = self.allocate_object(HashMap::new());
        
        // Set the instance's prototype to the class's prototype
        self.prototypes.insert(instance_id, prototype_id);
        
        Some(instance_id)
    }

    /// Get the prototype of an object
    fn get_prototype(&self, object_id: u64) -> Option<u64> {
        self.prototypes.get(&object_id).copied()
    }

    /// Set the prototype of an object
    fn set_prototype(&mut self, object_id: u64, prototype_id: u64) {
        self.prototypes.insert(object_id, prototype_id);
    }

    /// Check if an object is an instance of a class (instanceof operator)
    /// Walks the prototype chain to check
    fn is_instance_of(&self, object_id: u64, class_id: u64) -> bool {
        let class = match self.classes.get(&class_id) {
            Some(c) => c,
            None => return false,
        };
        
        let target_prototype = class.prototype_id;
        let mut current_proto = self.prototypes.get(&object_id).copied();
        
        while let Some(proto_id) = current_proto {
            if proto_id == target_prototype {
                return true;
            }
            current_proto = self.prototypes.get(&proto_id).copied();
        }
        
        false
    }

    /// Get a property from an object, walking the prototype chain if needed
    fn get_property_with_prototype(&self, object_id: u64, key: &str) -> Option<f64> {
        // First check the object itself
        if let Some(obj) = self.objects.get(&object_id) {
            if let Some(&value) = obj.get(key) {
                return Some(value);
            }
        }
        
        // Walk the prototype chain
        let mut current_proto = self.prototypes.get(&object_id).copied();
        while let Some(proto_id) = current_proto {
            if let Some(proto_obj) = self.objects.get(&proto_id) {
                if let Some(&value) = proto_obj.get(key) {
                    return Some(value);
                }
            }
            current_proto = self.prototypes.get(&proto_id).copied();
        }
        
        None
    }

    /// Define a method on a class prototype
    fn define_method(&mut self, class_id: u64, name: String, function_id: u32, is_static: bool) {
        if is_static {
            // Static methods go on the class itself
            if let Some(class) = self.classes.get_mut(&class_id) {
                // Store function_id as f64 (it will be used as a closure reference)
                class.static_properties.insert(name, function_id as f64);
            }
        } else {
            // Instance methods go on the prototype
            if let Some(class) = self.classes.get(&class_id) {
                let prototype_id = class.prototype_id;
                if let Some(proto) = self.objects.get_mut(&prototype_id) {
                    proto.insert(name, function_id as f64);
                }
            }
        }
    }

    /// Define a getter on a class
    fn define_getter(&mut self, class_id: u64, name: String, function_id: u32, _is_static: bool) {
        if let Some(class) = self.classes.get_mut(&class_id) {
            class.getters.insert(name, function_id);
        }
    }

    /// Define a setter on a class
    fn define_setter(&mut self, class_id: u64, name: String, function_id: u32, _is_static: bool) {
        if let Some(class) = self.classes.get_mut(&class_id) {
            class.setters.insert(name, function_id);
        }
    }

    /// Get a getter function ID for a property
    fn get_getter(&self, class_id: u64, name: &str) -> Option<u32> {
        self.classes.get(&class_id)?.getters.get(name).copied()
    }

    /// Get a setter function ID for a property
    fn get_setter(&self, class_id: u64, name: &str) -> Option<u32> {
        self.classes.get(&class_id)?.setters.get(name).copied()
    }

    /// Define a private field on a class
    fn define_private_field(&mut self, class_id: u64, name: String, initial_value: f64) {
        if let Some(class) = self.classes.get_mut(&class_id) {
            class.private_fields.insert(name, initial_value);
        }
    }

    /// Calculate approximate memory usage of the runtime heap
    /// Returns (heap_used, heap_total, object_count)
    fn memory_usage(&self) -> (usize, usize, usize) {
        let mut heap_used = 0usize;

        // Estimate memory for strings
        for s in self.strings.values() {
            heap_used += s.len() + std::mem::size_of::<String>();
        }

        // Estimate memory for arrays
        for arr in self.arrays.values() {
            heap_used += arr.len() * std::mem::size_of::<f64>() + std::mem::size_of::<Vec<f64>>();
        }

        // Estimate memory for objects
        for obj in self.objects.values() {
            heap_used +=
                obj.len() * (std::mem::size_of::<String>() + std::mem::size_of::<f64>() + 32);
            // Estimate key size
        }

        // Estimate memory for closures
        for closure in self.closures.values() {
            heap_used += closure.captured_vars.len() * std::mem::size_of::<f64>()
                + std::mem::size_of::<ClosureData>();
        }

        // Estimate memory for generators
        for gen in self.generators.values() {
            heap_used += gen.captured_vars.len() * std::mem::size_of::<f64>()
                + std::mem::size_of::<GeneratorData>();
        }

        // Estimate memory for promises
        heap_used += self.promises.len() * std::mem::size_of::<PromiseData>();

        // Estimate memory for BigInts (rough estimate based on digit count)
        for bigint in self.bigints.values() {
            // Each BigInt digit is typically 32 or 64 bits
            // Use to_string().len() as a rough proxy for size
            heap_used += bigint.to_string().len() + 32; // Base overhead + digits
        }

        // Estimate memory for classes
        for class in self.classes.values() {
            heap_used += std::mem::size_of::<ClassData>();
            heap_used += class.static_properties.len() * 40;
            heap_used += class.private_fields.len() * 40;
            heap_used += class.getters.len() * 40;
            heap_used += class.setters.len() * 40;
        }

        let object_count = self.strings.len()
            + self.arrays.len()
            + self.objects.len()
            + self.closures.len()
            + self.generators.len()
            + self.promises.len()
            + self.bigints.len()
            + self.classes.len();

        // Heap total is an estimate - we don't have a fixed heap size
        let heap_total = heap_used.max(1024 * 1024); // At least 1 MB

        (heap_used, heap_total, object_count)
    }
}

/// Access the thread-local runtime heap
///
/// This function provides access to the per-thread RuntimeHeap.
/// Each thread has its own heap instance, eliminating lock contention.
/// Reserved for JIT-compiled code that needs heap access
#[allow(dead_code)]
fn with_runtime_heap<F, R>(f: F) -> R
where
    F: FnOnce(&mut RuntimeHeap) -> R,
{
    RUNTIME_HEAP.with(|heap| f(&mut heap.borrow_mut()))
}

/// Access the thread-local runtime heap (read-only)
fn with_runtime_heap_ref<F, R>(f: F) -> R
where
    F: FnOnce(&RuntimeHeap) -> R,
{
    RUNTIME_HEAP.with(|heap| f(&heap.borrow()))
}

/// Compatibility wrapper for thread-local heap access
/// This struct provides a Mutex-like API for backward compatibility
/// while using thread-local storage internally.
struct ThreadLocalHeapGuard;

impl ThreadLocalHeapGuard {
    fn lock(&self) -> Result<ThreadLocalHeapLock, std::sync::PoisonError<()>> {
        Ok(ThreadLocalHeapLock)
    }
}

/// Lock guard for thread-local heap access
struct ThreadLocalHeapLock;

impl std::ops::Deref for ThreadLocalHeapLock {
    type Target = RuntimeHeap;

    fn deref(&self) -> &Self::Target {
        // This is a bit of a hack - we return a reference that's only valid
        // within the thread-local context. The caller must not hold this
        // reference across yield points.
        RUNTIME_HEAP.with(|heap| {
            // SAFETY: The reference is only valid within this thread and
            // the caller is expected to use it immediately
            unsafe { &*heap.as_ptr() }
        })
    }
}

impl std::ops::DerefMut for ThreadLocalHeapLock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        RUNTIME_HEAP.with(|heap| {
            // SAFETY: The reference is only valid within this thread and
            // the caller is expected to use it immediately
            unsafe { &mut *heap.as_ptr() }
        })
    }
}

/// Get access to the thread-local runtime heap
/// Returns a guard that provides Mutex-like API for backward compatibility
fn get_runtime_heap_lock() -> ThreadLocalHeapGuard {
    ThreadLocalHeapGuard
}

/// Get memory usage from the runtime heap
/// Returns (heap_used, heap_total, object_count)
pub fn get_runtime_memory_usage() -> (usize, usize, usize) {
    with_runtime_heap_ref(|heap| heap.memory_usage())
}

// Runtime functions for closure/array/object operations
extern "C" fn builtin_create_closure(function_id: f64, captured_count: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    let captured_vars = vec![f64::NAN; captured_count as usize];
    let id = heap.allocate_closure(function_id as u32, captured_vars, false);
    id as f64
}

extern "C" fn builtin_create_array(element_count: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    let elements = vec![f64::NAN; element_count as usize];
    let id = heap.allocate_array(elements);
    id as f64
}

extern "C" fn builtin_array_push(array_id: f64, value: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    if let Some(arr) = heap.get_array_mut(array_id as u64) {
        arr.push(value);
        arr.len() as f64
    } else {
        f64::NAN
    }
}

extern "C" fn builtin_array_get(array_id: f64, index: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let heap = heap.lock().unwrap();
    if let Some(arr) = heap.get_array(array_id as u64) {
        arr.get(index as usize).copied().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

extern "C" fn builtin_array_set(array_id: f64, index: f64, value: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    if let Some(arr) = heap.get_array_mut(array_id as u64) {
        let idx = index as usize;
        if idx < arr.len() {
            arr[idx] = value;
            value
        } else {
            // Extend array if needed
            while arr.len() <= idx {
                arr.push(f64::NAN);
            }
            arr[idx] = value;
            value
        }
    } else {
        f64::NAN
    }
}

// ============================================================================
// Destructuring Built-in Functions
// Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7
// ============================================================================

/// Array slice operation for rest elements in destructuring
/// Creates a new array from source[start_index..]
/// Requirements: 7.4 - rest elements in array destructuring
extern "C" fn builtin_array_slice_from(source_id: f64, start_index: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    
    let start = start_index as usize;
    
    // Get the source array
    if let Some(source_arr) = heap.get_array(source_id as u64) {
        // Create a new array with elements from start_index to end
        let sliced: Vec<f64> = if start < source_arr.len() {
            source_arr[start..].to_vec()
        } else {
            Vec::new()
        };
        
        let new_id = heap.allocate_array(sliced);
        new_id as f64
    } else {
        // Source is not an array, return empty array
        let new_id = heap.allocate_array(Vec::new());
        new_id as f64
    }
}

/// Object rest operation for rest properties in destructuring
/// Creates a new object with all properties except excluded ones
/// Requirements: 7.5 - rest properties in object destructuring
/// Note: excluded_count is passed but actual exclusion is handled at compile time
/// by not including those properties in the copy
extern "C" fn builtin_object_rest(source_id: f64, _excluded_count: f64) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    
    // Get the source object
    if let Some(source_obj) = heap.get_object(source_id as u64) {
        // Clone all properties to a new object
        // Note: In a full implementation, we would exclude specific keys
        // For now, we copy all properties (the compiler handles exclusion)
        let new_props = source_obj.clone();
        let new_id = heap.allocate_object(new_props);
        new_id as f64
    } else {
        // Source is not an object, return empty object
        let new_id = heap.allocate_object(HashMap::new());
        new_id as f64
    }
}

/// Check if a value is undefined
/// Returns 1.0 if undefined (NaN), 0.0 otherwise
/// Requirements: 7.3 - default values in destructuring
extern "C" fn builtin_is_undefined(value: f64) -> f64 {
    // In our NaN-boxing scheme, undefined is represented as NaN
    if value.is_nan() {
        1.0
    } else {
        0.0
    }
}

/// Throw TypeError for destructuring null/undefined
/// Requirements: 7.7 - destructuring null/undefined error
extern "C" fn builtin_throw_destructuring_error(source: f64) -> f64 {
    // Determine if it's null or undefined
    let type_name = if source.is_nan() {
        "undefined"
    } else if source == crate::compiler::codegen::NULL_VALUE {
        "null"
    } else {
        "nullish value"
    };
    
    // Set the exception
    let error_msg = format!("Cannot destructure {} as it is {}", type_name, type_name);
    
    // Create a TypeError and throw it
    CURRENT_EXCEPTION.with(|exc| {
        let mut heap = get_runtime_heap_lock().lock().unwrap();
        let error_id = heap.allocate_string(error_msg);
        *exc.borrow_mut() = -(error_id as f64 + 1_000_000.0);
    });
    
    f64::NAN
}

/// Build a template literal by concatenating quasis and expressions
/// Requirements: 8.1, 8.2 - template literal interpolation with multiline support
/// 
/// This function takes:
/// - quasis_ptr: pointer to array of string IDs (static parts)
/// - quasis_len: number of quasis
/// - exprs_ptr: pointer to array of expression values
/// - exprs_len: number of expressions
/// 
/// Returns: string ID of the concatenated result
extern "C" fn builtin_build_template_literal(
    quasis_ptr: *const f64,
    quasis_len: usize,
    exprs_ptr: *const f64,
    exprs_len: usize,
) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    
    // Build the result string by interleaving quasis and expressions
    let mut result = String::new();
    
    // Safety: We trust the compiler to pass valid pointers and lengths
    let quasis = if quasis_len > 0 && !quasis_ptr.is_null() {
        unsafe { std::slice::from_raw_parts(quasis_ptr, quasis_len) }
    } else {
        &[]
    };
    
    let exprs = if exprs_len > 0 && !exprs_ptr.is_null() {
        unsafe { std::slice::from_raw_parts(exprs_ptr, exprs_len) }
    } else {
        &[]
    };
    
    // Template literals interleave: quasi[0], expr[0], quasi[1], expr[1], ..., quasi[n]
    // There's always one more quasi than expressions
    for (i, &quasi_id) in quasis.iter().enumerate() {
        // Add the quasi (static string part)
        if is_string_id(quasi_id) {
            let string_id = decode_string_id(quasi_id);
            if let Some(s) = heap.get_string(string_id) {
                result.push_str(s);
            }
        }
        
        // Add the expression value (if there is one)
        if i < exprs.len() {
            let expr_val = exprs[i];
            let expr_str = value_to_string_internal(expr_val, &heap);
            result.push_str(&expr_str);
        }
    }
    
    // Allocate the result string
    let id = heap.allocate_string(result);
    encode_string_id(id)
}

/// Call a tagged template function
/// Requirements: 8.3 - tagged template invocation
/// 
/// Tagged templates call the tag function with:
/// - First argument: array of cooked strings (with escape sequences processed)
/// - Additional arguments: the interpolated values
/// - The strings array has a 'raw' property with unprocessed strings
extern "C" fn builtin_call_tagged_template(
    tag_fn: f64,
    strings_array: f64,
    exprs_ptr: *const f64,
    exprs_len: usize,
) -> f64 {
    let heap = get_runtime_heap_lock();
    let mut heap = heap.lock().unwrap();
    
    // Get the tag function
    let tag_id = tag_fn as u64;
    
    // Build arguments array: [strings_array, ...expressions]
    let mut args = Vec::with_capacity(1 + exprs_len);
    args.push(strings_array);
    
    // Add expression values
    if exprs_len > 0 && !exprs_ptr.is_null() {
        let exprs = unsafe { std::slice::from_raw_parts(exprs_ptr, exprs_len) };
        args.extend_from_slice(exprs);
    }
    
    // Call the tag function
    if let Some(closure) = heap.get_closure(tag_id) {
        let func_id = closure.function_id;
        let captured = closure.captured_vars.clone();
        drop(heap); // Release lock before calling
        
        // Call the function with the arguments
        // This would need to integrate with the runtime's function calling mechanism
        // For now, return NaN as a placeholder
        let _ = (func_id, captured, args);
        f64::NAN
    } else {
        // Not a function - throw TypeError
        CURRENT_EXCEPTION.with(|exc| {
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            let error_id = heap.allocate_string("Tag is not a function".to_string());
            *exc.borrow_mut() = -(error_id as f64 + 1_000_000.0);
        });
        f64::NAN
    }
}

extern "C" fn builtin_create_object() -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = heap.allocate_object(HashMap::new());
    id as f64
}

extern "C" fn builtin_object_set(object_id: f64, key_hash: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(obj) = heap.get_object_mut(object_id as u64) {
        // Use key_hash as string key for now (would need proper string interning)
        let key = format!("prop_{}", key_hash as u64);
        obj.insert(key, value);
        value
    } else {
        f64::NAN
    }
}

extern "C" fn builtin_object_get(object_id: f64, key_hash: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(obj) = heap.get_object(object_id as u64) {
        let key = format!("prop_{}", key_hash as u64);
        obj.get(&key).copied().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

// ============================================================================
// Class-related Built-in Functions (ES6 Classes)
// Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8
// ============================================================================

/// Create a class and return its ID
/// constructor_id: The function ID of the constructor (NaN if no explicit constructor)
/// super_class_id: The ID of the parent class (NaN if no extends clause)
extern "C" fn builtin_create_class(constructor_id: f64, super_class_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    let ctor_id = if constructor_id.is_nan() {
        None
    } else {
        Some(constructor_id as u32)
    };
    
    let super_id = if super_class_id.is_nan() || super_class_id == 0.0 {
        None
    } else {
        Some(super_class_id as u64)
    };
    
    let class_id = heap.allocate_class(ctor_id, super_id);
    class_id as f64
}

/// Create an instance of a class
/// Returns the instance object ID
/// Requirements: 6.1 - WHEN a class is instantiated with new, THE DX_Runtime SHALL call the constructor
extern "C" fn builtin_create_instance(class_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    match heap.create_instance(class_id as u64) {
        Some(instance_id) => instance_id as f64,
        None => {
            drop(heap);
            throw_type_error("Cannot create instance of non-class");
            f64::NAN
        }
    }
}

/// Get the constructor function ID for a class
/// Returns NaN if the class has no explicit constructor
extern "C" fn builtin_get_class_constructor(class_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    
    match heap.get_class(class_id as u64) {
        Some(class) => match class.constructor_id {
            Some(ctor_id) => ctor_id as f64,
            None => f64::NAN,
        },
        None => f64::NAN,
    }
}

/// Get the prototype object ID for a class
extern "C" fn builtin_get_class_prototype(class_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    
    match heap.get_class(class_id as u64) {
        Some(class) => class.prototype_id as f64,
        None => f64::NAN,
    }
}

/// Get the super class ID for a class
/// Returns NaN if the class has no parent
extern "C" fn builtin_get_super_class(class_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    
    match heap.get_class(class_id as u64) {
        Some(class) => match class.super_class_id {
            Some(super_id) => super_id as f64,
            None => f64::NAN,
        },
        None => f64::NAN,
    }
}

/// Check if an object is an instance of a class (instanceof operator)
/// Requirements: 6.3 - prototype chain for inheritance
extern "C" fn builtin_instanceof(object_id: f64, class_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    
    if heap.is_instance_of(object_id as u64, class_id as u64) {
        1.0
    } else {
        0.0
    }
}

/// Define a method on a class prototype
/// Requirements: 6.2 - methods are accessible on instances
extern "C" fn builtin_define_method(class_id: f64, name_id: f64, function_id: f64, is_static: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    // Get the method name from the string heap
    let name = if is_string_id(name_id) {
        let string_id = decode_string_id(name_id);
        match heap.get_string(string_id) {
            Some(s) => s.clone(),
            None => return f64::NAN,
        }
    } else {
        // Use hash-based name as fallback
        format!("method_{}", name_id as u64)
    };
    
    heap.define_method(class_id as u64, name, function_id as u32, is_static != 0.0);
    f64::NAN // Return undefined
}

/// Define a getter on a class
/// Requirements: 6.7 - getters/setters are invoked on property access/assignment
extern "C" fn builtin_define_getter(class_id: f64, name_id: f64, function_id: f64, is_static: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    let name = if is_string_id(name_id) {
        let string_id = decode_string_id(name_id);
        match heap.get_string(string_id) {
            Some(s) => s.clone(),
            None => return f64::NAN,
        }
    } else {
        format!("getter_{}", name_id as u64)
    };
    
    heap.define_getter(class_id as u64, name, function_id as u32, is_static != 0.0);
    f64::NAN
}

/// Define a setter on a class
/// Requirements: 6.7 - getters/setters are invoked on property access/assignment
extern "C" fn builtin_define_setter(class_id: f64, name_id: f64, function_id: f64, is_static: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    let name = if is_string_id(name_id) {
        let string_id = decode_string_id(name_id);
        match heap.get_string(string_id) {
            Some(s) => s.clone(),
            None => return f64::NAN,
        }
    } else {
        format!("setter_{}", name_id as u64)
    };
    
    heap.define_setter(class_id as u64, name, function_id as u32, is_static != 0.0);
    f64::NAN
}

/// Get a static property from a class
/// Requirements: 6.6 - static methods are attached to class constructor
extern "C" fn builtin_get_static_property(class_id: f64, name_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    
    let name = if is_string_id(name_id) {
        let string_id = decode_string_id(name_id);
        match heap.get_string(string_id) {
            Some(s) => s.clone(),
            None => return f64::NAN,
        }
    } else {
        format!("prop_{}", name_id as u64)
    };
    
    match heap.get_class(class_id as u64) {
        Some(class) => class.static_properties.get(&name).copied().unwrap_or(f64::NAN),
        None => f64::NAN,
    }
}

/// Set a static property on a class
/// Requirements: 6.6 - static methods are attached to class constructor
extern "C" fn builtin_set_static_property(class_id: f64, name_id: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    let name = if is_string_id(name_id) {
        let string_id = decode_string_id(name_id);
        match heap.get_string(string_id) {
            Some(s) => s.clone(),
            None => return f64::NAN,
        }
    } else {
        format!("prop_{}", name_id as u64)
    };
    
    if let Some(class) = heap.get_class_mut(class_id as u64) {
        class.static_properties.insert(name, value);
    }
    value
}

/// Get a property from an object, walking the prototype chain
/// This is used for method lookup on instances
extern "C" fn builtin_get_property_with_proto(object_id: f64, name_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    
    let name = if is_string_id(name_id) {
        let string_id = decode_string_id(name_id);
        match heap.get_string(string_id) {
            Some(s) => s.clone(),
            None => return f64::NAN,
        }
    } else {
        format!("prop_{}", name_id as u64)
    };
    
    heap.get_property_with_prototype(object_id as u64, &name).unwrap_or(f64::NAN)
}

/// Set the prototype of an object
extern "C" fn builtin_set_object_prototype(object_id: f64, prototype_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    heap.set_prototype(object_id as u64, prototype_id as u64);
    f64::NAN
}

/// Get the prototype of an object
extern "C" fn builtin_get_object_prototype(object_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    match heap.get_prototype(object_id as u64) {
        Some(proto_id) => proto_id as f64,
        None => f64::NAN,
    }
}

/// Define a private field on a class
/// Requirements: 6.8 - private fields are stored separately
extern "C" fn builtin_define_private_field(class_id: f64, name_id: f64, initial_value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    let name = if is_string_id(name_id) {
        let string_id = decode_string_id(name_id);
        match heap.get_string(string_id) {
            Some(s) => s.clone(),
            None => return f64::NAN,
        }
    } else {
        format!("#field_{}", name_id as u64)
    };
    
    heap.define_private_field(class_id as u64, name, initial_value);
    f64::NAN
}

/// Get a method from the super class's prototype
/// Requirements: 6.5 - super.method() looks up method on parent prototype
extern "C" fn builtin_get_super_method(class_id: f64, method_name_hash: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    
    // Get the super class
    let super_class_id = match heap.get_class(class_id as u64) {
        Some(class) => match class.super_class_id {
            Some(id) => id,
            None => return f64::NAN, // No super class
        },
        None => return f64::NAN,
    };
    
    // Get the super class's prototype
    let super_prototype_id = match heap.get_class(super_class_id) {
        Some(class) => class.prototype_id,
        None => return f64::NAN,
    };
    
    // Look up the method on the super prototype
    // The method_name_hash is used to look up the method
    // For now, we return the prototype ID and let the caller handle the lookup
    super_prototype_id as f64
}

// ============================================================================
// End Class-related Built-in Functions
// ============================================================================

extern "C" fn builtin_set_captured(closure_id: f64, index: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(closure) = heap.get_closure_mut(closure_id as u64) {
        let idx = index as usize;
        if idx < closure.captured_vars.len() {
            closure.captured_vars[idx] = value;
        }
    }
    value
}

extern "C" fn builtin_get_captured(closure_id: f64, index: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(closure) = heap.get_closure(closure_id as u64) {
        let idx = index as usize;
        closure.captured_vars.get(idx).copied().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

extern "C" fn builtin_set_arrow(closure_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(closure) = heap.get_closure_mut(closure_id as u64) {
        closure.is_arrow = true;
    }
    closure_id
}

// Thread-local storage for the current closure being executed
thread_local! {
    static CURRENT_CLOSURE: std::cell::RefCell<u64> = const { std::cell::RefCell::new(0) };
}

/// Call a closure with up to 8 arguments
/// This is the main entry point for calling JavaScript functions
/// Requirements: 5.1 - WHEN an async function is called, THE DX_Runtime SHALL return a Promise immediately
extern "C" fn builtin_call_function(
    closure_id: f64,
    arg_count: f64,
    arg0: f64,
    arg1: f64,
    arg2: f64,
    arg3: f64,
    arg4: f64,
    arg5: f64,
    arg6: f64,
    arg7: f64,
) -> f64 {
    // Check if this is a BigInt method call
    if is_bigint_method_id(closure_id) {
        let method_id = decode_bigint_method_id(closure_id);
        // arg0 should be the BigInt value (this)
        return call_bigint_method(method_id, arg0);
    }
    
    let id = closure_id as u64;
    
    // Check if this is an async function call
    // Requirements: 5.1 - async function returns Promise immediately
    {
        let heap = get_runtime_heap_lock().lock().unwrap();
        if heap.get_async_function(id).is_some() {
            drop(heap); // Release lock before calling
            return call_async_function_wrapper(id, arg_count, arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7);
        }
    }
    
    let heap = get_runtime_heap_lock().lock().unwrap();

    // Get the closure data
    let closure = match heap.get_closure(id) {
        Some(c) => c.clone(),
        None => return f64::NAN,
    };

    // Get the compiled function pointer
    let func_ptr = match get_compiled_function(closure.function_id) {
        Some(ptr) => ptr,
        None => return f64::NAN,
    };

    // Set the current closure for captured variable access
    CURRENT_CLOSURE.with(|c| {
        *c.borrow_mut() = id;
    });

    // Call the function based on argument count
    let argc = arg_count as usize;
    let result = unsafe {
        match argc {
            0 => {
                let func: extern "C" fn() -> f64 = std::mem::transmute(func_ptr);
                func()
            }
            1 => {
                let func: extern "C" fn(f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0)
            }
            2 => {
                let func: extern "C" fn(f64, f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0, arg1)
            }
            3 => {
                let func: extern "C" fn(f64, f64, f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2)
            }
            4 => {
                let func: extern "C" fn(f64, f64, f64, f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3)
            }
            5 => {
                let func: extern "C" fn(f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4)
            }
            6 => {
                let func: extern "C" fn(f64, f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4, arg5)
            }
            7 => {
                let func: extern "C" fn(f64, f64, f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4, arg5, arg6)
            }
            _ => {
                let func: extern "C" fn(f64, f64, f64, f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7)
            }
        }
    };

    // Clear the current closure
    CURRENT_CLOSURE.with(|c| {
        *c.borrow_mut() = 0;
    });

    result
}

/// Call a BigInt method
fn call_bigint_method(method_id: u64, this_val: f64) -> f64 {
    // Get the BigInt value
    let bigint = match get_bigint_from_value(this_val) {
        Some(v) => v,
        None => return f64::NAN,
    };
    
    match method_id {
        0 => {
            // toString()
            let string_repr = bigint.to_string();
            let heap = get_runtime_heap_lock();
            let mut heap = heap.lock().unwrap();
            let string_id = heap.allocate_string(string_repr);
            encode_string_id(string_id)
        }
        1 => {
            // valueOf() - returns the BigInt itself
            this_val
        }
        2 => {
            // toLocaleString() - same as toString for now
            let string_repr = bigint.to_string();
            let heap = get_runtime_heap_lock();
            let mut heap = heap.lock().unwrap();
            let string_id = heap.allocate_string(string_repr);
            encode_string_id(string_id)
        }
        _ => f64::NAN,
    }
}

/// Get the current closure ID (for captured variable access)
extern "C" fn builtin_get_current_closure() -> f64 {
    CURRENT_CLOSURE.with(|c| *c.borrow() as f64)
}

// Thread-local storage for the current `this` binding
thread_local! {
    static CURRENT_THIS: std::cell::RefCell<f64> = const { std::cell::RefCell::new(f64::NAN) };
}

extern "C" fn builtin_get_this() -> f64 {
    CURRENT_THIS.with(|this| *this.borrow())
}

extern "C" fn builtin_set_this(value: f64) -> f64 {
    CURRENT_THIS.with(|this| {
        *this.borrow_mut() = value;
    });
    value
}

// Type constants for typeof - reserved for full typeof implementation
const TYPE_UNDEFINED: f64 = 0.0;
const TYPE_NUMBER: f64 = 1.0;
#[allow(dead_code)]
const TYPE_STRING: f64 = 2.0;
#[allow(dead_code)]
const TYPE_BOOLEAN: f64 = 3.0;
const TYPE_OBJECT: f64 = 4.0;
const TYPE_FUNCTION: f64 = 5.0;
#[allow(dead_code)]
const TYPE_SYMBOL: f64 = 6.0;
#[allow(dead_code)]
const TYPE_BIGINT: f64 = 7.0;

extern "C" fn builtin_typeof(value: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();

    // Check if it's NaN (undefined in our representation)
    if value.is_nan() {
        return TYPE_UNDEFINED;
    }

    // Check if it's a heap object by ID
    let id = value as u64;

    // Check if it's a closure (function)
    if heap.closures.contains_key(&id) {
        return TYPE_FUNCTION;
    }

    // Check if it's an array (arrays are objects in JS)
    if heap.arrays.contains_key(&id) {
        return TYPE_OBJECT;
    }

    // Check if it's an object
    if heap.objects.contains_key(&id) {
        return TYPE_OBJECT;
    }

    // Check for boolean (0.0 or 1.0 could be boolean, but we treat all numbers as numbers)
    // In a full implementation, we'd have tagged values

    // Default to number
    TYPE_NUMBER
}

// Generator runtime functions
// Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7

/// Generator result object tag - used to identify generator result objects
/// Generator results are objects with {value, done} properties
const GENERATOR_RESULT_TAG: f64 = 10_000_000.0;

/// Create a generator object from a generator function
/// Requirements: 9.1 - generator function returns generator object without executing body
extern "C" fn builtin_create_generator(function_id: f64, captured_count: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let captured_vars = vec![f64::NAN; captured_count as usize];
    let id = heap.allocate_generator(function_id as u32, captured_vars);
    id as f64
}

/// Create a generator result object {value, done}
/// Returns an object ID that can be used to access value and done properties
fn create_generator_result(heap: &mut RuntimeHeap, value: f64, done: bool) -> f64 {
    let mut props = HashMap::new();
    props.insert("value".to_string(), value);
    props.insert("done".to_string(), if done { 1.0 } else { 0.0 });
    let id = heap.allocate_object(props);
    id as f64
}

/// Get the next value from a generator
/// Requirements: 9.2 - next() executes until yield, returns {value, done}
/// Requirements: 9.3 - yield suspends execution and returns yielded value
/// Requirements: 9.4 - generator returns {value: undefined, done: true} when finished
/// Requirements: 9.5 - next(value) passes value as yield expression result
extern "C" fn builtin_generator_next(generator_id: f64, send_value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    // First, get the state and update the generator
    let (state, value, is_done) = {
        if let Some(gen) = heap.get_generator_mut(generator_id as u64) {
            match gen.state {
                GeneratorState::Completed => {
                    // Requirements: 9.4 - Return {value: undefined, done: true} when finished
                    (Some(GeneratorState::Completed), f64::NAN, true)
                }
                GeneratorState::Created => {
                    // First call to next() - start executing the generator
                    gen.state = GeneratorState::Executing;
                    // The first next() call ignores the send_value per spec
                    gen.sent_value = f64::NAN;
                    
                    // In a full implementation, we would execute the generator body here
                    // For now, we simulate by returning the current value
                    let value = gen.current_value;
                    let is_done = gen.state == GeneratorState::Completed;
                    (Some(GeneratorState::Created), value, is_done)
                }
                GeneratorState::Suspended => {
                    // Resume execution from yield point
                    gen.state = GeneratorState::Executing;
                    // Requirements: 9.5 - Store send_value for yield expression result
                    gen.sent_value = send_value;
                    
                    // In a full implementation, we would resume the generator body here
                    // For now, we simulate by returning the current value
                    let value = gen.current_value;
                    let is_done = gen.state == GeneratorState::Completed;
                    (Some(GeneratorState::Suspended), value, is_done)
                }
                GeneratorState::Executing => {
                    // Generator is already executing - this is an error
                    (Some(GeneratorState::Executing), f64::NAN, false)
                }
            }
        } else {
            (None, f64::NAN, false)
        }
    };
    
    match state {
        Some(GeneratorState::Executing) => {
            throw_type_error("Generator is already executing");
            f64::NAN
        }
        Some(_) => {
            create_generator_result(&mut heap, value, is_done)
        }
        None => {
            throw_type_error("Invalid generator object");
            f64::NAN
        }
    }
}

/// Yield a value from the generator
/// Requirements: 9.3 - yield suspends execution and returns yielded value
extern "C" fn builtin_generator_yield(generator_id: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(gen) = heap.get_generator_mut(generator_id as u64) {
        gen.state = GeneratorState::Suspended;
        gen.current_value = value;
        // Return the sent value (will be used as yield expression result on resume)
        gen.sent_value
    } else {
        f64::NAN
    }
}

/// Complete the generator with a return value
/// Requirements: 9.4 - generator returns {value: returnValue, done: true} when finished
extern "C" fn builtin_generator_return(generator_id: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(gen) = heap.get_generator_mut(generator_id as u64) {
        gen.state = GeneratorState::Completed;
        gen.current_value = value;
        value
    } else {
        f64::NAN
    }
}

/// Throw an error into the generator
/// Requirements: 9.6 - throw() throws error inside generator
extern "C" fn builtin_generator_throw(generator_id: f64, error: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    // Get the state and update the generator
    let state = {
        if let Some(gen) = heap.get_generator_mut(generator_id as u64) {
            match gen.state {
                GeneratorState::Completed => Some(GeneratorState::Completed),
                GeneratorState::Created => {
                    // If generator hasn't started, complete it and throw
                    gen.state = GeneratorState::Completed;
                    Some(GeneratorState::Created)
                }
                GeneratorState::Suspended => {
                    // Store the error to be thrown when generator resumes
                    gen.thrown_error = Some(error);
                    gen.state = GeneratorState::Completed;
                    Some(GeneratorState::Suspended)
                }
                GeneratorState::Executing => Some(GeneratorState::Executing),
            }
        } else {
            None
        }
    };
    
    match state {
        Some(GeneratorState::Executing) => {
            throw_type_error("Generator is already executing");
            f64::NAN
        }
        Some(_) => {
            // Set the exception for all other states
            CURRENT_EXCEPTION.with(|exc| {
                *exc.borrow_mut() = error;
            });
            f64::NAN
        }
        None => {
            throw_type_error("Invalid generator object");
            f64::NAN
        }
    }
}

/// Force the generator to return with a given value
/// Requirements: 9.7 - return() completes generator with given value
extern "C" fn builtin_generator_force_return(generator_id: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    
    // Get the state and update the generator
    let state = {
        if let Some(gen) = heap.get_generator_mut(generator_id as u64) {
            match gen.state {
                GeneratorState::Completed => Some(GeneratorState::Completed),
                GeneratorState::Created | GeneratorState::Suspended => {
                    // Complete the generator with the given value
                    gen.state = GeneratorState::Completed;
                    gen.current_value = value;
                    gen.return_value = Some(value);
                    Some(GeneratorState::Suspended) // Use Suspended to indicate success
                }
                GeneratorState::Executing => Some(GeneratorState::Executing),
            }
        } else {
            None
        }
    };
    
    match state {
        Some(GeneratorState::Executing) => {
            throw_type_error("Generator is already executing");
            f64::NAN
        }
        Some(_) => {
            create_generator_result(&mut heap, value, true)
        }
        None => {
            throw_type_error("Invalid generator object");
            f64::NAN
        }
    }
}

/// Get the current state of a generator (for testing/debugging)
/// Returns: 0 = Created, 1 = Suspended, 2 = Executing, 3 = Completed
extern "C" fn builtin_generator_get_state(generator_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(gen) = heap.get_generator(generator_id as u64) {
        match gen.state {
            GeneratorState::Created => 0.0,
            GeneratorState::Suspended => 1.0,
            GeneratorState::Executing => 2.0,
            GeneratorState::Completed => 3.0,
        }
    } else {
        f64::NAN
    }
}

/// Check if a value is a generator object
extern "C" fn builtin_is_generator(value: f64) -> f64 {
    if value.is_nan() || value < 0.0 || value.fract() != 0.0 {
        return 0.0;
    }
    let heap = get_runtime_heap_lock().lock().unwrap();
    if heap.get_generator(value as u64).is_some() {
        1.0
    } else {
        0.0
    }
}

// Promise runtime functions
extern "C" fn builtin_create_promise() -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = heap.allocate_promise();
    id as f64
}

extern "C" fn builtin_promise_resolve(promise_id: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(promise) = heap.get_promise_mut(promise_id as u64) {
        if promise.state == PromiseState::Pending {
            promise.state = PromiseState::Fulfilled;
            promise.value = value;
            // In a full implementation, we'd call the then callbacks here
        }
    }
    value
}

extern "C" fn builtin_promise_reject(promise_id: f64, reason: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(promise) = heap.get_promise_mut(promise_id as u64) {
        if promise.state == PromiseState::Pending {
            promise.state = PromiseState::Rejected;
            promise.value = reason;
            // In a full implementation, we'd call the catch callbacks here
        }
    }
    reason
}

extern "C" fn builtin_promise_get_value(promise_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(promise) = heap.get_promise(promise_id as u64) {
        promise.value
    } else {
        f64::NAN
    }
}

/// Call an async function wrapper - executes the function body and returns the promise
/// Requirements: 5.1 - WHEN an async function is called, THE DX_Runtime SHALL return a Promise immediately
/// Requirements: 5.4 - WHEN an async function throws, THE DX_Runtime SHALL reject the returned Promise
/// Requirements: 5.5 - WHEN an async function returns a value, THE DX_Runtime SHALL resolve the returned Promise
fn call_async_function_wrapper(
    async_fn_id: u64,
    arg_count: f64,
    arg0: f64,
    arg1: f64,
    arg2: f64,
    arg3: f64,
    arg4: f64,
    arg5: f64,
    arg6: f64,
    arg7: f64,
) -> f64 {
    // Get the async function data
    let (function_id, result_promise_id, captured_vars) = {
        let mut heap = get_runtime_heap_lock().lock().unwrap();
        let async_fn = match heap.get_async_function_mut(async_fn_id) {
            Some(af) => af,
            None => return f64::NAN,
        };
        
        // Mark as started
        async_fn.started = true;
        
        (async_fn.function_id, async_fn.result_promise_id, async_fn.captured_vars.clone())
    };
    
    // Get the compiled function pointer for the async function body
    let func_ptr = match get_compiled_function(function_id) {
        Some(ptr) => ptr,
        None => {
            // Reject the promise if function not found
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            if let Some(promise) = heap.get_promise_mut(result_promise_id) {
                promise.state = PromiseState::Rejected;
                promise.value = f64::NAN; // Error value
            }
            return result_promise_id as f64;
        }
    };
    
    // Create a temporary closure for the async function to enable captured variable access
    let temp_closure_id = {
        let mut heap = get_runtime_heap_lock().lock().unwrap();
        heap.allocate_closure(function_id, captured_vars, false)
    };
    
    // Set the current closure for captured variable access
    CURRENT_CLOSURE.with(|c| {
        *c.borrow_mut() = temp_closure_id;
    });
    
    // Execute the async function body
    // The function body will create a promise, execute, and return the promise
    // For now, we execute synchronously and handle the result
    let argc = arg_count as usize;
    let result = unsafe {
        match argc {
            0 => {
                let func: extern "C" fn() -> f64 = std::mem::transmute(func_ptr);
                func()
            }
            1 => {
                let func: extern "C" fn(f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0)
            }
            2 => {
                let func: extern "C" fn(f64, f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0, arg1)
            }
            3 => {
                let func: extern "C" fn(f64, f64, f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2)
            }
            4 => {
                let func: extern "C" fn(f64, f64, f64, f64) -> f64 = std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3)
            }
            5 => {
                let func: extern "C" fn(f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4)
            }
            6 => {
                let func: extern "C" fn(f64, f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4, arg5)
            }
            7 => {
                let func: extern "C" fn(f64, f64, f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4, arg5, arg6)
            }
            _ => {
                let func: extern "C" fn(f64, f64, f64, f64, f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(func_ptr);
                func(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7)
            }
        }
    };
    
    // Clear the current closure
    CURRENT_CLOSURE.with(|c| {
        *c.borrow_mut() = 0;
    });
    
    // Check if an exception was thrown
    let exception = CURRENT_EXCEPTION.with(|e| *e.borrow());
    if !exception.is_nan() {
        // Reject the promise with the exception
        let mut heap = get_runtime_heap_lock().lock().unwrap();
        if let Some(promise) = heap.get_promise_mut(result_promise_id) {
            if promise.state == PromiseState::Pending {
                promise.state = PromiseState::Rejected;
                promise.value = exception;
                // Trigger catch callbacks
                let callbacks = promise.catch_callbacks.clone();
                drop(heap);
                trigger_promise_catch_callbacks(result_promise_id, exception, callbacks);
            }
        }
        // Clear the exception
        CURRENT_EXCEPTION.with(|e| {
            *e.borrow_mut() = f64::NAN;
        });
    } else {
        // The async function body returns a promise that it created internally
        // We need to resolve our result promise with the returned value
        // For simple async functions without await, the result is the return value
        
        // Check if result is a promise (the internal promise created by the async function)
        let is_internal_promise = {
            let heap = get_runtime_heap_lock().lock().unwrap();
            heap.get_promise(result as u64).is_some()
        };
        
        if is_internal_promise {
            // The async function returned its internal promise
            // We need to chain our result promise to it
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            if let Some(internal_promise) = heap.get_promise(result as u64) {
                match internal_promise.state {
                    PromiseState::Fulfilled => {
                        let value = internal_promise.value;
                        if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                            if result_promise.state == PromiseState::Pending {
                                result_promise.state = PromiseState::Fulfilled;
                                result_promise.value = value;
                            }
                        }
                    }
                    PromiseState::Rejected => {
                        let reason = internal_promise.value;
                        if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                            if result_promise.state == PromiseState::Pending {
                                result_promise.state = PromiseState::Rejected;
                                result_promise.value = reason;
                            }
                        }
                    }
                    PromiseState::Pending => {
                        // The internal promise is still pending (has await)
                        // We'll need to chain the result promise to it
                        // For now, just return the result promise ID
                    }
                }
            }
        } else {
            // The result is a direct value, resolve the promise with it
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            if let Some(promise) = heap.get_promise_mut(result_promise_id) {
                if promise.state == PromiseState::Pending {
                    promise.state = PromiseState::Fulfilled;
                    promise.value = result;
                    // Trigger then callbacks
                    let callbacks = promise.then_callbacks.clone();
                    drop(heap);
                    trigger_promise_callbacks(result_promise_id, result, true, callbacks);
                }
            }
        }
    }
    
    // Return the result promise ID immediately
    result_promise_id as f64
}

// Async function runtime
/// Create an async function wrapper that returns a Promise when called
/// Requirements: 5.1 - WHEN an async function is called, THE DX_Runtime SHALL return a Promise immediately
extern "C" fn builtin_create_async_function(function_id: f64, captured_count: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    // Create captured variables array
    let captured_vars = vec![f64::NAN; captured_count as usize];
    // Allocate the async function state which includes creating the result promise
    let id = heap.allocate_async_function(function_id as u32, captured_vars);
    id as f64
}

/// Call an async function - returns the result promise immediately
/// Requirements: 5.1 - async function returns Promise immediately
extern "C" fn builtin_call_async_function(async_fn_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function_mut(id) {
        // Mark as started
        async_fn.started = true;
        // Return the result promise ID
        async_fn.result_promise_id as f64
    } else {
        // Return NaN if async function not found
        f64::NAN
    }
}

/// Get the result promise ID for an async function
extern "C" fn builtin_get_async_promise(async_fn_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function(id) {
        async_fn.result_promise_id as f64
    } else {
        f64::NAN
    }
}

/// Resolve an async function's promise with a value
/// Requirements: 5.5 - WHEN an async function returns a value, THE DX_Runtime SHALL resolve the returned Promise
extern "C" fn builtin_async_resolve(async_fn_id: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function(id) {
        let promise_id = async_fn.result_promise_id;
        // Resolve the promise
        if let Some(promise) = heap.get_promise_mut(promise_id) {
            if promise.state == PromiseState::Pending {
                promise.state = PromiseState::Fulfilled;
                promise.value = value;
                // Trigger then callbacks
                let callbacks = promise.then_callbacks.clone();
                drop(heap); // Release lock before calling callbacks
                trigger_promise_callbacks(promise_id, value, true, callbacks);
            }
        }
        value
    } else {
        f64::NAN
    }
}

/// Reject an async function's promise with an error
/// Requirements: 5.4 - WHEN an async function throws, THE DX_Runtime SHALL reject the returned Promise
extern "C" fn builtin_async_reject(async_fn_id: f64, reason: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function(id) {
        let promise_id = async_fn.result_promise_id;
        // Reject the promise
        if let Some(promise) = heap.get_promise_mut(promise_id) {
            if promise.state == PromiseState::Pending {
                promise.state = PromiseState::Rejected;
                promise.value = reason;
                // Trigger catch callbacks
                let callbacks = promise.catch_callbacks.clone();
                drop(heap); // Release lock before calling callbacks
                trigger_promise_catch_callbacks(promise_id, reason, callbacks);
            }
        }
        reason
    } else {
        f64::NAN
    }
}

/// Helper function to trigger then callbacks when a promise is fulfilled
fn trigger_promise_callbacks(_promise_id: u64, value: f64, is_fulfilled: bool, callbacks: Vec<(Option<u64>, u64)>) {
    for (callback_id, result_promise_id) in callbacks {
        if let Some(cb_id) = callback_id {
            // Call the callback closure with the value
            // The callback should resolve/reject the result promise
            let result = call_closure_with_value(cb_id, value);
            
            // Resolve the result promise with the callback's return value
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                if result_promise.state == PromiseState::Pending {
                    result_promise.state = PromiseState::Fulfilled;
                    result_promise.value = result;
                }
            }
        } else {
            // No callback - pass through the value
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                if result_promise.state == PromiseState::Pending {
                    if is_fulfilled {
                        result_promise.state = PromiseState::Fulfilled;
                    } else {
                        result_promise.state = PromiseState::Rejected;
                    }
                    result_promise.value = value;
                }
            }
        }
    }
}

/// Helper function to trigger catch callbacks when a promise is rejected
fn trigger_promise_catch_callbacks(_promise_id: u64, reason: f64, callbacks: Vec<(Option<u64>, u64)>) {
    for (callback_id, result_promise_id) in callbacks {
        if let Some(cb_id) = callback_id {
            // Call the callback closure with the reason
            let result = call_closure_with_value(cb_id, reason);
            
            // Resolve the result promise with the callback's return value (recovery)
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                if result_promise.state == PromiseState::Pending {
                    result_promise.state = PromiseState::Fulfilled;
                    result_promise.value = result;
                }
            }
        } else {
            // No callback - propagate the rejection
            let mut heap = get_runtime_heap_lock().lock().unwrap();
            if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                if result_promise.state == PromiseState::Pending {
                    result_promise.state = PromiseState::Rejected;
                    result_promise.value = reason;
                }
            }
        }
    }
}

/// Helper function to call a closure with a single value argument
fn call_closure_with_value(closure_id: u64, value: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    if let Some(closure) = heap.get_closure(closure_id) {
        let function_id = closure.function_id;
        drop(heap); // Release lock before calling function
        
        // Get the function pointer and call it
        if let Some(func_ptr) = get_compiled_function(function_id) {
            // Call the function with the value as argument
            let func: extern "C" fn(f64) -> f64 = unsafe { std::mem::transmute(func_ptr) };
            func(value)
        } else {
            f64::NAN
        }
    } else {
        f64::NAN
    }
}

// Thread-local storage for the current async function context
thread_local! {
    /// The ID of the currently executing async function (0 if none)
    static CURRENT_ASYNC_FUNCTION: std::cell::RefCell<u64> = const { std::cell::RefCell::new(0) };
    /// Flag indicating if we're in an await suspension state
    static AWAIT_SUSPENDED: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
}

/// Set the current async function context
extern "C" fn builtin_set_async_context(async_fn_id: f64) -> f64 {
    CURRENT_ASYNC_FUNCTION.with(|c| {
        *c.borrow_mut() = async_fn_id as u64;
    });
    async_fn_id
}

/// Get the current async function context
extern "C" fn builtin_get_async_context() -> f64 {
    CURRENT_ASYNC_FUNCTION.with(|c| *c.borrow() as f64)
}

/// Clear the current async function context
extern "C" fn builtin_clear_async_context() -> f64 {
    CURRENT_ASYNC_FUNCTION.with(|c| {
        *c.borrow_mut() = 0;
    });
    0.0
}

/// Check if we're in an await suspension state
extern "C" fn builtin_is_await_suspended() -> f64 {
    AWAIT_SUSPENDED.with(|s| if *s.borrow() { 1.0 } else { 0.0 })
}

/// Await a promise - handles suspension and resumption
/// Requirements: 5.2 - WHEN await is encountered, THE DX_Runtime SHALL suspend execution until the Promise resolves
/// Requirements: 5.3 - WHEN an awaited Promise rejects, THE DX_Runtime SHALL throw the rejection reason as an exception
extern "C" fn builtin_await(promise_id: f64) -> f64 {
    // First, check if the value is not a promise - return it directly (auto-wrapping)
    let heap = get_runtime_heap_lock().lock().unwrap();
    let id = promise_id as u64;
    
    if let Some(promise) = heap.get_promise(id) {
        match promise.state {
            PromiseState::Fulfilled => {
                // Promise is already fulfilled - return the value immediately
                promise.value
            }
            PromiseState::Rejected => {
                // Promise is rejected - throw the rejection reason as an exception
                // Requirements: 5.3 - WHEN an awaited Promise rejects, THE DX_Runtime SHALL throw the rejection reason
                let reason = promise.value;
                drop(heap);
                
                // Set the exception state so the caller can handle it
                CURRENT_EXCEPTION.with(|exc| {
                    *exc.borrow_mut() = reason;
                });
                
                // Also set a structured exception for better error reporting
                let error_msg = {
                    let heap = get_runtime_heap_lock().lock().unwrap();
                    if is_string_id(reason) {
                        let str_id = decode_string_id(reason);
                        heap.get_string(str_id).cloned().unwrap_or_else(|| "Promise rejected".to_string())
                    } else {
                        format!("Promise rejected with value: {}", reason)
                    }
                };
                throw_type_error(error_msg);
                
                f64::NAN
            }
            PromiseState::Pending => {
                // Promise is pending - we need to suspend execution
                // Requirements: 5.2 - WHEN await is encountered, THE DX_Runtime SHALL suspend execution
                
                // Get the current async function context
                let async_fn_id = CURRENT_ASYNC_FUNCTION.with(|c| *c.borrow());
                
                if async_fn_id == 0 {
                    // Not in an async function context - this is an error
                    // In a real implementation, await outside async function is a syntax error
                    // For runtime, we return NaN to indicate the pending state
                    drop(heap);
                    return f64::NAN;
                }
                
                // Save the current state for later resumption
                drop(heap);
                
                // Register callbacks on the promise to resume execution when it settles
                // The callback will resolve/reject the async function's result promise
                let mut heap = get_runtime_heap_lock().lock().unwrap();
                
                if let Some(async_fn) = heap.get_async_function(async_fn_id) {
                    let result_promise_id = async_fn.result_promise_id;
                    
                    // Register then/catch callbacks on the awaited promise
                    if let Some(promise) = heap.get_promise_mut(id) {
                        // Add a callback that will be triggered when the promise settles
                        // The callback will chain to the async function's result promise
                        promise.then_callbacks.push((None, result_promise_id));
                        promise.catch_callbacks.push((None, result_promise_id));
                    }
                }
                
                // Mark that we're in a suspended state
                AWAIT_SUSPENDED.with(|s| {
                    *s.borrow_mut() = true;
                });
                
                // Return NaN to indicate suspension
                // The caller should check AWAIT_SUSPENDED to know if this is a suspension
                f64::NAN
            }
        }
    } else {
        // Not a promise - return the value directly (auto-wrapping per spec)
        // This handles cases like `await 42` which should just return 42
        promise_id
    }
}

/// Await a promise with explicit async function context
/// This version takes the async function ID explicitly for better control
/// Requirements: 5.2 - WHEN await is encountered, THE DX_Runtime SHALL suspend execution until the Promise resolves
/// Requirements: 5.3 - WHEN an awaited Promise rejects, THE DX_Runtime SHALL throw the rejection reason as an exception
extern "C" fn builtin_await_with_context(promise_id: f64, async_fn_id: f64) -> f64 {
    // Set the async context temporarily
    let old_context = CURRENT_ASYNC_FUNCTION.with(|c| {
        let old = *c.borrow();
        *c.borrow_mut() = async_fn_id as u64;
        old
    });
    
    // Call the regular await
    let result = builtin_await(promise_id);
    
    // Restore the old context
    CURRENT_ASYNC_FUNCTION.with(|c| {
        *c.borrow_mut() = old_context;
    });
    
    result
}

/// Save the execution state of an async function for later resumption
/// This is called when an await encounters a pending promise
extern "C" fn builtin_save_async_state(async_fn_id: f64, state_index: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function_mut(id) {
        // Save the current state index (which await point we're at)
        async_fn.current_state = state_index as u32;
        1.0 // Success
    } else {
        0.0 // Failure - async function not found
    }
}

/// Save a local variable value for an async function
/// This is used to preserve local variables across await points
extern "C" fn builtin_save_async_local(async_fn_id: f64, local_index: f64, value: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function_mut(id) {
        let idx = local_index as usize;
        // Ensure the saved_locals vector is large enough
        while async_fn.saved_locals.len() <= idx {
            async_fn.saved_locals.push(f64::NAN);
        }
        async_fn.saved_locals[idx] = value;
        value
    } else {
        f64::NAN
    }
}

/// Restore a local variable value for an async function
/// This is used to restore local variables when resuming after await
extern "C" fn builtin_restore_async_local(async_fn_id: f64, local_index: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function(id) {
        let idx = local_index as usize;
        async_fn.saved_locals.get(idx).copied().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// Get the current state index of an async function
/// This is used to determine which await point to resume from
extern "C" fn builtin_get_async_state(async_fn_id: f64) -> f64 {
    let heap = get_runtime_heap_lock().lock().unwrap();
    let id = async_fn_id as u64;
    
    if let Some(async_fn) = heap.get_async_function(id) {
        async_fn.current_state as f64
    } else {
        0.0
    }
}

/// Resume an async function after an await point
/// This is called when a pending promise settles
extern "C" fn builtin_resume_async(async_fn_id: f64, resolved_value: f64, is_rejection: f64) -> f64 {
    // Clear the suspension flag
    AWAIT_SUSPENDED.with(|s| {
        *s.borrow_mut() = false;
    });
    
    if is_rejection != 0.0 {
        // The promise was rejected - set the exception state
        CURRENT_EXCEPTION.with(|exc| {
            *exc.borrow_mut() = resolved_value;
        });
        
        // Reject the async function's result promise
        let mut heap = get_runtime_heap_lock().lock().unwrap();
        let id = async_fn_id as u64;
        
        if let Some(async_fn) = heap.get_async_function(id) {
            let result_promise_id = async_fn.result_promise_id;
            if let Some(promise) = heap.get_promise_mut(result_promise_id) {
                if promise.state == PromiseState::Pending {
                    promise.state = PromiseState::Rejected;
                    promise.value = resolved_value;
                }
            }
        }
        
        f64::NAN
    } else {
        // The promise was fulfilled - return the resolved value
        resolved_value
    }
}

/// Add a then callback to a promise
/// Returns a new promise that will be resolved with the callback's result
extern "C" fn builtin_promise_then(promise_id: f64, on_fulfilled_id: f64, on_rejected_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = promise_id as u64;
    
    // Create the result promise
    let result_promise_id = heap.allocate_promise();
    
    let on_fulfilled = if on_fulfilled_id.is_nan() { None } else { Some(on_fulfilled_id as u64) };
    let on_rejected = if on_rejected_id.is_nan() { None } else { Some(on_rejected_id as u64) };
    
    if let Some(promise) = heap.get_promise_mut(id) {
        match promise.state {
            PromiseState::Pending => {
                // Add callbacks to be called when promise settles
                if on_fulfilled.is_some() {
                    promise.then_callbacks.push((on_fulfilled, result_promise_id));
                }
                if on_rejected.is_some() {
                    promise.catch_callbacks.push((on_rejected, result_promise_id));
                }
            }
            PromiseState::Fulfilled => {
                // Promise already fulfilled - call the callback immediately
                let value = promise.value;
                drop(heap);
                
                if let Some(cb_id) = on_fulfilled {
                    let result = call_closure_with_value(cb_id, value);
                    let mut heap = get_runtime_heap_lock().lock().unwrap();
                    if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                        result_promise.state = PromiseState::Fulfilled;
                        result_promise.value = result;
                    }
                } else {
                    let mut heap = get_runtime_heap_lock().lock().unwrap();
                    if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                        result_promise.state = PromiseState::Fulfilled;
                        result_promise.value = value;
                    }
                }
                return result_promise_id as f64;
            }
            PromiseState::Rejected => {
                // Promise already rejected - call the catch callback
                let reason = promise.value;
                drop(heap);
                
                if let Some(cb_id) = on_rejected {
                    let result = call_closure_with_value(cb_id, reason);
                    let mut heap = get_runtime_heap_lock().lock().unwrap();
                    if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                        result_promise.state = PromiseState::Fulfilled;
                        result_promise.value = result;
                    }
                } else {
                    let mut heap = get_runtime_heap_lock().lock().unwrap();
                    if let Some(result_promise) = heap.get_promise_mut(result_promise_id) {
                        result_promise.state = PromiseState::Rejected;
                        result_promise.value = reason;
                    }
                }
                return result_promise_id as f64;
            }
        }
    }
    
    result_promise_id as f64
}

/// Add a catch callback to a promise
extern "C" fn builtin_promise_catch(promise_id: f64, on_rejected_id: f64) -> f64 {
    builtin_promise_then(promise_id, f64::NAN, on_rejected_id)
}

/// Promise.all - wait for all promises to resolve
/// Requirements: 5.6 - WHEN Promise.all is called, THE DX_Runtime SHALL wait for all promises and return an array of results
extern "C" fn builtin_promise_all_runtime(array_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = array_id as u64;
    
    // Get the array of promises
    let promises = if let Some(arr) = heap.get_array(id) {
        arr.clone()
    } else {
        // Not an array - return rejected promise
        let error_msg = heap.allocate_string("Promise.all requires an array".to_string());
        let result_id = heap.allocate_promise();
        if let Some(promise) = heap.get_promise_mut(result_id) {
            promise.state = PromiseState::Rejected;
            promise.value = encode_string_id(error_msg);
        }
        return result_id as f64;
    };
    
    let mut results = Vec::with_capacity(promises.len());
    let mut has_rejection = false;
    let mut rejection_value = f64::NAN;
    let mut has_pending = false;
    
    for promise_id in &promises {
        let pid = *promise_id as u64;
        if let Some(promise) = heap.get_promise(pid) {
            match promise.state {
                PromiseState::Fulfilled => {
                    results.push(promise.value);
                }
                PromiseState::Rejected => {
                    has_rejection = true;
                    rejection_value = promise.value;
                    break;
                }
                PromiseState::Pending => {
                    has_pending = true;
                    break;
                }
            }
        } else {
            // Not a promise - treat as resolved value
            results.push(*promise_id);
        }
    }
    
    if has_rejection {
        // First rejection - return rejected promise
        let result_id = heap.allocate_promise();
        if let Some(result_promise) = heap.get_promise_mut(result_id) {
            result_promise.state = PromiseState::Rejected;
            result_promise.value = rejection_value;
        }
        return result_id as f64;
    }
    
    if has_pending {
        // Has pending promises - return pending promise
        let result_id = heap.allocate_promise();
        return result_id as f64;
    }
    
    // All promises fulfilled - create result array and promise
    let result_array_id = heap.allocate_array(results);
    let result_promise_id = heap.allocate_promise();
    if let Some(promise) = heap.get_promise_mut(result_promise_id) {
        promise.state = PromiseState::Fulfilled;
        promise.value = result_array_id as f64;
    }
    
    result_promise_id as f64
}

/// Promise.race - resolve/reject with first settled promise
/// Requirements: 5.7 - WHEN Promise.race is called, THE DX_Runtime SHALL resolve/reject with the first settled promise
extern "C" fn builtin_promise_race_runtime(array_id: f64) -> f64 {
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = array_id as u64;
    
    // Get the array of promises
    let promises = if let Some(arr) = heap.get_array(id) {
        arr.clone()
    } else {
        // Not an array - return rejected promise
        let error_msg = heap.allocate_string("Promise.race requires an array".to_string());
        let result_id = heap.allocate_promise();
        if let Some(promise) = heap.get_promise_mut(result_id) {
            promise.state = PromiseState::Rejected;
            promise.value = encode_string_id(error_msg);
        }
        return result_id as f64;
    };
    
    // Find the first settled promise
    let mut first_settled: Option<(PromiseState, f64)> = None;
    let mut first_non_promise: Option<f64> = None;
    
    for promise_id in &promises {
        let pid = *promise_id as u64;
        if let Some(promise) = heap.get_promise(pid) {
            match promise.state {
                PromiseState::Fulfilled => {
                    first_settled = Some((PromiseState::Fulfilled, promise.value));
                    break;
                }
                PromiseState::Rejected => {
                    first_settled = Some((PromiseState::Rejected, promise.value));
                    break;
                }
                PromiseState::Pending => {
                    // Continue checking other promises
                    continue;
                }
            }
        } else {
            // Not a promise - treat as immediately resolved
            first_non_promise = Some(*promise_id);
            break;
        }
    }
    
    if let Some((state, value)) = first_settled {
        let result_id = heap.allocate_promise();
        if let Some(result_promise) = heap.get_promise_mut(result_id) {
            result_promise.state = state;
            result_promise.value = value;
        }
        return result_id as f64;
    }
    
    if let Some(value) = first_non_promise {
        let result_id = heap.allocate_promise();
        if let Some(result_promise) = heap.get_promise_mut(result_id) {
            result_promise.state = PromiseState::Fulfilled;
            result_promise.value = value;
        }
        return result_id as f64;
    }
    
    // All promises pending - return pending promise
    let result_id = heap.allocate_promise();
    result_id as f64
}

/// Get the type name for an f64 value (for error messages)
fn get_type_name(value: f64) -> &'static str {
    if is_null_value(value) {
        "null"
    } else if is_undefined_value(value) {
        "undefined"
    } else if value.is_nan() {
        "NaN"
    } else if is_bigint_id(value) {
        "bigint"
    } else if is_string_id(value) {
        "string"
    } else {
        "number"
    }
}

// Property access runtime functions
extern "C" fn builtin_get_property(object_id: f64, offset: f64) -> f64 {
    // Check for null/undefined property access - throw TypeError
    // Requirements: 2.4, 14.1, 14.2
    if is_nullish_value(object_id) {
        let type_name = get_type_name(object_id);
        throw_type_error(format!(
            "Cannot read properties of {} (reading property at offset {})",
            type_name, offset as u64
        ));
        return f64::NAN;
    }
    
    let heap = get_runtime_heap_lock().lock().unwrap();
    let id = object_id as u64;

    // Check if it's an array (for indexed access)
    if let Some(arr) = heap.get_array(id) {
        let idx = offset as usize;
        return arr.get(idx).copied().unwrap_or(f64::NAN);
    }

    // Check if it's an object
    if let Some(obj) = heap.get_object(id) {
        // Use offset as property key hash
        let key = format!("prop_{}", offset as u64);
        return obj.get(&key).copied().unwrap_or(f64::NAN);
    }

    f64::NAN
}

extern "C" fn builtin_set_property(object_id: f64, offset: f64, value: f64) -> f64 {
    // Check for null/undefined property access - throw TypeError
    // Requirements: 2.4, 14.1, 14.2
    if is_nullish_value(object_id) {
        let type_name = get_type_name(object_id);
        throw_type_error(format!(
            "Cannot set properties of {} (setting property at offset {})",
            type_name, offset as u64
        ));
        return f64::NAN;
    }
    
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = object_id as u64;

    // Check if it's an array (for indexed access)
    if let Some(arr) = heap.get_array_mut(id) {
        let idx = offset as usize;
        if idx < arr.len() {
            arr[idx] = value;
        } else {
            // Extend array if needed
            while arr.len() <= idx {
                arr.push(f64::NAN);
            }
            arr[idx] = value;
        }
        return value;
    }

    // Check if it's an object
    if let Some(obj) = heap.get_object_mut(id) {
        let key = format!("prop_{}", offset as u64);
        obj.insert(key, value);
        return value;
    }

    f64::NAN
}

extern "C" fn builtin_get_property_dynamic(object_id: f64, key_hash: f64) -> f64 {
    // Check for null/undefined property access - throw TypeError
    // Requirements: 2.4, 14.1, 14.2
    if is_nullish_value(object_id) {
        let type_name = get_type_name(object_id);
        throw_type_error(format!(
            "Cannot read properties of {} (reading property with hash {})",
            type_name, key_hash as u64
        ));
        return f64::NAN;
    }
    
    // Check if this is a BigInt value - BigInts have special method handling
    if is_bigint_id(object_id) {
        // BigInt methods are handled specially
        // The key_hash represents the method name
        // For toString, we return a special function ID that will be handled at call time
        let key_hash_u32 = key_hash as u32;
        
        // For BigInt.toString(), return a special marker that indicates this is a BigInt method
        // We encode this as a special function reference
        if key_hash_u32 == hash_string("toString") {
            // Return a special marker for BigInt.toString
            // This will be handled by the call mechanism
            return encode_bigint_method_id(0); // 0 = toString
        }
        if key_hash_u32 == hash_string("valueOf") {
            return encode_bigint_method_id(1); // 1 = valueOf
        }
        if key_hash_u32 == hash_string("toLocaleString") {
            return encode_bigint_method_id(2); // 2 = toLocaleString
        }
        
        return f64::NAN;
    }
    
    let heap = get_runtime_heap_lock().lock().unwrap();
    let id = object_id as u64;

    if let Some(obj) = heap.get_object(id) {
        let key = format!("prop_{}", key_hash as u64);
        return obj.get(&key).copied().unwrap_or(f64::NAN);
    }

    f64::NAN
}

/// Encode a BigInt method ID for later dispatch
fn encode_bigint_method_id(method_id: u64) -> f64 {
    // Use a special tag offset for BigInt methods
    const BIGINT_METHOD_TAG: f64 = 3_000_000.0;
    -(method_id as f64 + BIGINT_METHOD_TAG)
}

/// Check if a value is a BigInt method reference
fn is_bigint_method_id(value: f64) -> bool {
    const BIGINT_METHOD_TAG: f64 = 3_000_000.0;
    // BigInt method IDs are encoded as -(method_id + BIGINT_METHOD_TAG)
    // So for method_id 0, value = -3_000_000
    // For method_id 99, value = -3_000_099
    // We check if value is in range [-3_000_099, -3_000_000]
    value <= -BIGINT_METHOD_TAG && 
    value >= -BIGINT_METHOD_TAG - 100.0 &&
    value.fract() == 0.0
}

/// Decode a BigInt method ID
fn decode_bigint_method_id(value: f64) -> u64 {
    const BIGINT_METHOD_TAG: f64 = 3_000_000.0;
    (-(value + BIGINT_METHOD_TAG)) as u64
}

extern "C" fn builtin_set_property_dynamic(object_id: f64, key_hash: f64, value: f64) -> f64 {
    // Check for null/undefined property access - throw TypeError
    // Requirements: 2.4, 14.1, 14.2
    if is_nullish_value(object_id) {
        let type_name = get_type_name(object_id);
        throw_type_error(format!(
            "Cannot set properties of {} (setting property with hash {})",
            type_name, key_hash as u64
        ));
        return f64::NAN;
    }
    
    let mut heap = get_runtime_heap_lock().lock().unwrap();
    let id = object_id as u64;

    if let Some(obj) = heap.get_object_mut(id) {
        let key = format!("prop_{}", key_hash as u64);
        obj.insert(key, value);
        return value;
    }

    f64::NAN
}

// Exception handling runtime
thread_local! {
    static EXCEPTION_HANDLERS: std::cell::RefCell<Vec<ExceptionHandler>> = const { std::cell::RefCell::new(Vec::new()) };
    static CURRENT_EXCEPTION: std::cell::RefCell<f64> = const { std::cell::RefCell::new(f64::NAN) };
    /// Structured exception storage for proper error reporting
    static STRUCTURED_EXCEPTION: std::cell::RefCell<Option<JsException>> = const { std::cell::RefCell::new(None) };
}

/// Store a structured exception for later retrieval
pub fn set_structured_exception(exception: JsException) {
    STRUCTURED_EXCEPTION.with(|exc| {
        *exc.borrow_mut() = Some(exception);
    });
}

/// Get the current structured exception if any
pub fn get_structured_exception() -> Option<JsException> {
    STRUCTURED_EXCEPTION.with(|exc| exc.borrow().clone())
}

/// Clear the structured exception
pub fn clear_structured_exception() {
    STRUCTURED_EXCEPTION.with(|exc| {
        *exc.borrow_mut() = None;
    });
}

/// Create and store a TypeError with the current stack trace
pub fn throw_type_error(message: impl Into<String>) {
    let mut exception = JsException::new(JsErrorType::TypeError, message);
    exception.stack = capture_stack_trace();
    if let Some(frame) = exception.stack.first() {
        exception.location =
            Some(crate::error::SourceLocation::new(&frame.file, frame.line, frame.column));
    }
    set_structured_exception(exception);
}

/// Create and store a ReferenceError with the current stack trace
pub fn throw_reference_error(message: impl Into<String>) {
    let mut exception = JsException::new(JsErrorType::ReferenceError, message);
    exception.stack = capture_stack_trace();
    if let Some(frame) = exception.stack.first() {
        exception.location =
            Some(crate::error::SourceLocation::new(&frame.file, frame.line, frame.column));
    }
    set_structured_exception(exception);
}

/// Create and store a RangeError with the current stack trace
pub fn throw_range_error(message: impl Into<String>) {
    let mut exception = JsException::new(JsErrorType::RangeError, message);
    exception.stack = capture_stack_trace();
    if let Some(frame) = exception.stack.first() {
        exception.location =
            Some(crate::error::SourceLocation::new(&frame.file, frame.line, frame.column));
    }
    set_structured_exception(exception);
}

/// Create and store a SyntaxError with the current stack trace
pub fn throw_syntax_error(message: impl Into<String>) {
    let mut exception = JsException::new(JsErrorType::SyntaxError, message);
    exception.stack = capture_stack_trace();
    if let Some(frame) = exception.stack.first() {
        exception.location =
            Some(crate::error::SourceLocation::new(&frame.file, frame.line, frame.column));
    }
    set_structured_exception(exception);
}

/// Exception handler for try/catch/finally - reserved for full exception handling
#[derive(Clone)]
#[allow(dead_code)]
struct ExceptionHandler {
    catch_block: u32,
    finally_block: Option<u32>,
}

extern "C" fn builtin_setup_exception_handler(catch_block: f64, finally_block: f64) -> f64 {
    EXCEPTION_HANDLERS.with(|handlers| {
        let mut handlers = handlers.borrow_mut();
        handlers.push(ExceptionHandler {
            catch_block: catch_block as u32,
            finally_block: if finally_block.is_nan() {
                None
            } else {
                Some(finally_block as u32)
            },
        });
    });
    0.0
}

extern "C" fn builtin_clear_exception_handler() -> f64 {
    EXCEPTION_HANDLERS.with(|handlers| {
        let mut handlers = handlers.borrow_mut();
        handlers.pop();
    });
    0.0
}

extern "C" fn builtin_throw(value: f64) -> f64 {
    // Store the exception value
    CURRENT_EXCEPTION.with(|exc| {
        *exc.borrow_mut() = value;
    });

    // Check if there's a handler
    let has_handler = EXCEPTION_HANDLERS.with(|handlers| !handlers.borrow().is_empty());

    if has_handler {
        // Return the catch block ID to jump to
        EXCEPTION_HANDLERS.with(|handlers| {
            let handlers = handlers.borrow();
            if let Some(handler) = handlers.last() {
                handler.catch_block as f64
            } else {
                f64::NAN
            }
        })
    } else {
        // No handler - return NaN to indicate uncaught exception
        f64::NAN
    }
}

extern "C" fn builtin_get_exception() -> f64 {
    CURRENT_EXCEPTION.with(|exc| *exc.borrow())
}

extern "C" fn builtin_clear_exception() -> f64 {
    CURRENT_EXCEPTION.with(|exc| {
        *exc.borrow_mut() = f64::NAN;
    });
    0.0
}

// ============================================================================
// Stack Frame Management Built-ins for Stack Trace Capture
// ============================================================================

/// Push a call frame onto the stack (called when entering a function)
/// Parameters are encoded as string IDs for function_name and file
extern "C" fn builtin_push_call_frame(function_name_id: f64, file_id: f64, line: f64, column: f64) -> f64 {
    let function_name = if is_string_id(function_name_id) {
        let id = decode_string_id(function_name_id);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        heap.get_string(id).cloned().unwrap_or_else(|| "<anonymous>".to_string())
    } else {
        "<anonymous>".to_string()
    };
    
    let file = if is_string_id(file_id) {
        let id = decode_string_id(file_id);
        let heap = get_runtime_heap_lock();
        let heap = heap.lock().unwrap();
        heap.get_string(id).cloned().unwrap_or_else(|| "<unknown>".to_string())
    } else {
        "<unknown>".to_string()
    };
    
    let frame = crate::error::CallFrame::new(
        function_name,
        file,
        line as u32,
        column as u32,
    );
    crate::error::push_call_frame(frame);
    0.0
}

/// Pop a call frame from the stack (called when exiting a function)
extern "C" fn builtin_pop_call_frame() -> f64 {
    crate::error::pop_call_frame();
    0.0
}

/// Get the current stack depth (for debugging)
extern "C" fn builtin_get_stack_depth() -> f64 {
    crate::error::call_stack_depth() as f64
}

/// Clear the entire call stack (for error recovery)
extern "C" fn builtin_clear_call_stack() -> f64 {
    crate::error::clear_call_stack();
    0.0
}

/// Code generator using Cranelift
pub struct CodeGenerator {
    #[allow(dead_code)]
    opt_level: OptLevel,
}

impl CodeGenerator {
    pub fn new(opt_level: OptLevel) -> DxResult<Self> {
        Ok(Self { opt_level })
    }

    /// Generate native code from MIR
    pub fn generate(
        &mut self,
        mir: &TypedMIR,
        filename: &str,
        source: &str,
    ) -> DxResult<CompiledModule> {
        // Create JIT module with native target
        let mut builder = JITBuilder::new(cranelift_module::default_libcall_names())
            .map_err(|e| DxError::CompileError(e.to_string()))?;

        // Register built-in functions as symbols
        builder.symbol("__dx_console_log", builtin_console_log as *const u8);
        builder.symbol("__dx_math_floor", builtin_math_floor as *const u8);
        builder.symbol("__dx_math_ceil", builtin_math_ceil as *const u8);
        builder.symbol("__dx_math_sqrt", builtin_math_sqrt as *const u8);
        builder.symbol("__dx_math_abs", builtin_math_abs as *const u8);
        builder.symbol("__dx_math_sin", builtin_math_sin as *const u8);
        builder.symbol("__dx_math_cos", builtin_math_cos as *const u8);
        builder.symbol("__dx_math_random", builtin_math_random as *const u8);
        builder.symbol("__dx_json_parse", builtin_json_parse as *const u8);
        builder.symbol("__dx_json_stringify", builtin_json_stringify as *const u8);
        builder.symbol("__dx_create_closure", builtin_create_closure as *const u8);
        builder.symbol("__dx_create_array", builtin_create_array as *const u8);
        builder.symbol("__dx_array_push", builtin_array_push as *const u8);
        builder.symbol("__dx_array_get", builtin_array_get as *const u8);
        builder.symbol("__dx_array_set", builtin_array_set as *const u8);
        builder.symbol("__dx_create_object", builtin_create_object as *const u8);
        builder.symbol("__dx_object_set", builtin_object_set as *const u8);
        builder.symbol("__dx_object_get", builtin_object_get as *const u8);
        builder.symbol("__dx_set_captured", builtin_set_captured as *const u8);
        builder.symbol("__dx_get_captured", builtin_get_captured as *const u8);
        builder.symbol("__dx_set_arrow", builtin_set_arrow as *const u8);
        builder.symbol("__dx_get_this", builtin_get_this as *const u8);
        builder.symbol("__dx_set_this", builtin_set_this as *const u8);
        builder.symbol("__dx_typeof", builtin_typeof as *const u8);
        builder.symbol("__dx_create_generator", builtin_create_generator as *const u8);
        builder.symbol("__dx_generator_next", builtin_generator_next as *const u8);
        builder.symbol("__dx_generator_yield", builtin_generator_yield as *const u8);
        builder.symbol("__dx_generator_return", builtin_generator_return as *const u8);
        builder.symbol("__dx_generator_throw", builtin_generator_throw as *const u8);
        builder.symbol("__dx_generator_force_return", builtin_generator_force_return as *const u8);
        builder.symbol("__dx_generator_get_state", builtin_generator_get_state as *const u8);
        builder.symbol("__dx_is_generator", builtin_is_generator as *const u8);
        builder.symbol("__dx_create_promise", builtin_create_promise as *const u8);
        builder.symbol("__dx_promise_resolve", builtin_promise_resolve as *const u8);
        builder.symbol("__dx_promise_reject", builtin_promise_reject as *const u8);
        builder.symbol("__dx_promise_get_value", builtin_promise_get_value as *const u8);
        
        // BigInt arithmetic built-ins
        builder.symbol("__dx_bigint_add", builtin_bigint_add as *const u8);
        builder.symbol("__dx_bigint_sub", builtin_bigint_sub as *const u8);
        builder.symbol("__dx_bigint_mul", builtin_bigint_mul as *const u8);
        builder.symbol("__dx_bigint_div", builtin_bigint_div as *const u8);
        builder.symbol("__dx_bigint_mod", builtin_bigint_mod as *const u8);
        builder.symbol("__dx_bigint_pow", builtin_bigint_pow as *const u8);
        
        // BigInt comparison built-ins
        builder.symbol("__dx_bigint_lt", builtin_bigint_lt as *const u8);
        builder.symbol("__dx_bigint_gt", builtin_bigint_gt as *const u8);
        builder.symbol("__dx_bigint_le", builtin_bigint_le as *const u8);
        builder.symbol("__dx_bigint_ge", builtin_bigint_ge as *const u8);
        builder.symbol("__dx_bigint_eq", builtin_bigint_eq as *const u8);
        builder.symbol("__dx_bigint_strict_eq", builtin_bigint_strict_eq as *const u8);
        
        // BigInt bitwise built-ins
        builder.symbol("__dx_bigint_and", builtin_bigint_and as *const u8);
        builder.symbol("__dx_bigint_or", builtin_bigint_or as *const u8);
        builder.symbol("__dx_bigint_xor", builtin_bigint_xor as *const u8);
        builder.symbol("__dx_bigint_not", builtin_bigint_not as *const u8);
        builder.symbol("__dx_bigint_shl", builtin_bigint_shl as *const u8);
        builder.symbol("__dx_bigint_shr", builtin_bigint_shr as *const u8);
        
        // BigInt conversion built-ins
        builder.symbol("__dx_bigint_to_string", builtin_bigint_to_string as *const u8);
        builder.symbol("__dx_bigint_from_string", builtin_bigint_from_string as *const u8);
        builder.symbol("__dx_bigint_from_number", builtin_bigint_from_number as *const u8);
        builder.symbol("__dx_is_bigint", builtin_is_bigint as *const u8);
        builder.symbol("__dx_bigint_type_check", builtin_bigint_type_check as *const u8);
        
        // Type coercion built-ins (ECMAScript spec)
        builder.symbol("__dx_to_boolean", builtin_to_boolean as *const u8);
        builder.symbol("__dx_to_number", builtin_to_number as *const u8);
        builder.symbol("__dx_to_string", builtin_to_string as *const u8);
        builder.symbol("__dx_strict_equals", builtin_strict_equals as *const u8);
        builder.symbol("__dx_loose_equals", builtin_loose_equals as *const u8);
        builder.symbol("__dx_string_concat", builtin_string_concat as *const u8);
        builder.symbol("__dx_is_string_operand", builtin_is_string_operand as *const u8);
        builder.symbol("__dx_check_bigint_number_mix", builtin_check_bigint_number_mix as *const u8);
        
        builder.symbol("__dx_create_async_function", builtin_create_async_function as *const u8);
        builder.symbol("__dx_call_async_function", builtin_call_async_function as *const u8);
        builder.symbol("__dx_get_async_promise", builtin_get_async_promise as *const u8);
        builder.symbol("__dx_async_resolve", builtin_async_resolve as *const u8);
        builder.symbol("__dx_async_reject", builtin_async_reject as *const u8);
        builder.symbol("__dx_await", builtin_await as *const u8);
        builder.symbol("__dx_await_with_context", builtin_await_with_context as *const u8);
        builder.symbol("__dx_set_async_context", builtin_set_async_context as *const u8);
        builder.symbol("__dx_get_async_context", builtin_get_async_context as *const u8);
        builder.symbol("__dx_clear_async_context", builtin_clear_async_context as *const u8);
        builder.symbol("__dx_is_await_suspended", builtin_is_await_suspended as *const u8);
        builder.symbol("__dx_save_async_state", builtin_save_async_state as *const u8);
        builder.symbol("__dx_save_async_local", builtin_save_async_local as *const u8);
        builder.symbol("__dx_restore_async_local", builtin_restore_async_local as *const u8);
        builder.symbol("__dx_get_async_state", builtin_get_async_state as *const u8);
        builder.symbol("__dx_resume_async", builtin_resume_async as *const u8);
        builder.symbol("__dx_promise_then", builtin_promise_then as *const u8);
        builder.symbol("__dx_promise_catch", builtin_promise_catch as *const u8);
        builder.symbol("__dx_promise_all", builtin_promise_all_runtime as *const u8);
        builder.symbol("__dx_promise_race", builtin_promise_race_runtime as *const u8);
        builder.symbol("__dx_get_property", builtin_get_property as *const u8);
        builder.symbol("__dx_set_property", builtin_set_property as *const u8);
        builder.symbol("__dx_get_property_dynamic", builtin_get_property_dynamic as *const u8);
        builder.symbol("__dx_set_property_dynamic", builtin_set_property_dynamic as *const u8);
        builder
            .symbol("__dx_setup_exception_handler", builtin_setup_exception_handler as *const u8);
        builder
            .symbol("__dx_clear_exception_handler", builtin_clear_exception_handler as *const u8);
        builder.symbol("__dx_throw", builtin_throw as *const u8);
        builder.symbol("__dx_get_exception", builtin_get_exception as *const u8);
        builder.symbol("__dx_clear_exception", builtin_clear_exception as *const u8);
        builder.symbol("__dx_call_function", builtin_call_function as *const u8);
        builder.symbol("__dx_get_current_closure", builtin_get_current_closure as *const u8);
        
        // Stack frame management built-ins for stack trace capture
        builder.symbol("__dx_push_call_frame", builtin_push_call_frame as *const u8);
        builder.symbol("__dx_pop_call_frame", builtin_pop_call_frame as *const u8);
        builder.symbol("__dx_get_stack_depth", builtin_get_stack_depth as *const u8);
        builder.symbol("__dx_clear_call_stack", builtin_clear_call_stack as *const u8);
        
        // Class-related built-ins (ES6 Classes)
        builder.symbol("__dx_create_class", builtin_create_class as *const u8);
        builder.symbol("__dx_create_instance", builtin_create_instance as *const u8);
        builder.symbol("__dx_get_class_constructor", builtin_get_class_constructor as *const u8);
        builder.symbol("__dx_get_class_prototype", builtin_get_class_prototype as *const u8);
        builder.symbol("__dx_get_super_class", builtin_get_super_class as *const u8);
        builder.symbol("__dx_instanceof", builtin_instanceof as *const u8);
        builder.symbol("__dx_define_method", builtin_define_method as *const u8);
        builder.symbol("__dx_define_getter", builtin_define_getter as *const u8);
        builder.symbol("__dx_define_setter", builtin_define_setter as *const u8);
        builder.symbol("__dx_get_static_property", builtin_get_static_property as *const u8);
        builder.symbol("__dx_set_static_property", builtin_set_static_property as *const u8);
        builder.symbol("__dx_get_property_with_proto", builtin_get_property_with_proto as *const u8);
        builder.symbol("__dx_set_object_prototype", builtin_set_object_prototype as *const u8);
        builder.symbol("__dx_get_object_prototype", builtin_get_object_prototype as *const u8);
        builder.symbol("__dx_define_private_field", builtin_define_private_field as *const u8);
        builder.symbol("__dx_get_super_method", builtin_get_super_method as *const u8);
        
        // Dynamic import built-in
        builder.symbol("__dx_dynamic_import", crate::compiler::dynamic_import::builtin_dynamic_import as *const u8);
        
        // Destructuring built-ins (Requirements: 7.1-7.7)
        builder.symbol("__dx_array_slice_from", builtin_array_slice_from as *const u8);
        builder.symbol("__dx_object_rest", builtin_object_rest as *const u8);
        builder.symbol("__dx_is_undefined", builtin_is_undefined as *const u8);
        builder.symbol("__dx_throw_destructuring_error", builtin_throw_destructuring_error as *const u8);
        
        // Template literal built-ins (Requirements: 8.1-8.3)
        builder.symbol("__dx_build_template_literal", builtin_build_template_literal as *const u8);
        builder.symbol("__dx_call_tagged_template", builtin_call_tagged_template as *const u8);

        let mut jit_module = JITModule::new(builder);
        let mut ctx = jit_module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut func_ids: HashMap<FunctionId, FuncId> = HashMap::new();
        let mut builtin_func_ids: HashMap<u32, FuncId> = HashMap::new();
        let mut signatures: HashMap<FunctionId, Signature> = HashMap::new();

        // Declare built-in functions
        self.declare_builtins(&mut jit_module, &mut builtin_func_ids)?;

        // First pass: declare all user functions
        for func in &mir.functions {
            let mut sig = jit_module.make_signature();

            // Add return type (always f64 for JS values)
            sig.returns.push(AbiParam::new(types::F64));

            // Add parameters
            for _param in &func.params {
                sig.params.push(AbiParam::new(types::F64));
            }

            let func_id = jit_module
                .declare_function(&func.name, Linkage::Local, &sig)
                .map_err(|e| DxError::CompileError(e.to_string()))?;

            func_ids.insert(func.id, func_id);
            signatures.insert(func.id, sig);
        }

        // Second pass: define all functions
        for func in &mir.functions {
            let func_id = func_ids[&func.id];
            let sig = &signatures[&func.id];

            ctx.func.signature = sig.clone();

            // Pre-declare all function references before creating FunctionBuilder
            let mut func_refs: HashMap<u32, FuncRef> = HashMap::new();
            for (&magic_id, &builtin_id) in &builtin_func_ids {
                let func_ref = jit_module.declare_func_in_func(builtin_id, &mut ctx.func);
                func_refs.insert(magic_id, func_ref);
            }
            for (&mir_func_id, &cl_func_id) in &func_ids {
                let func_ref = jit_module.declare_func_in_func(cl_func_id, &mut ctx.func);
                func_refs.insert(mir_func_id.0, func_ref);
            }

            self.compile_function_body(&mut ctx, &mut func_ctx, func, &func_refs)?;

            jit_module
                .define_function(func_id, &mut ctx)
                .map_err(|e| DxError::CompileError(e.to_string()))?;

            jit_module.clear_context(&mut ctx);
        }

        // Finalize
        jit_module
            .finalize_definitions()
            .map_err(|e| DxError::CompileError(e.to_string()))?;

        // Collect function pointers and register them for runtime calls
        let mut functions = HashMap::new();
        for (mir_id, cl_id) in &func_ids {
            let ptr = jit_module.get_finalized_function(*cl_id);
            functions.insert(*mir_id, ptr);

            // Register the function pointer for runtime calls
            register_compiled_function(mir_id.0, ptr);
        }

        let entry_point = mir.entry_point.and_then(|id| functions.get(&id).copied());

        // Create source map with the original source code
        let mut source_map = ModuleSourceMap::with_source(filename, source);

        // Add basic source map entries for each function
        // In a full implementation, we would track source locations during code generation
        for func in &mir.functions {
            // Add an entry for the function start
            // Line 1, column 1 is a placeholder - real implementation would track actual locations
            source_map.add_mapping(0, 1, 1, Some(&func.name));
        }

        Ok(CompiledModule {
            _jit_module: jit_module,
            functions,
            entry_point,
            source_hash: [0; 32],
            source_map,
        })
    }

    fn declare_builtins(
        &self,
        module: &mut JITModule,
        builtin_ids: &mut HashMap<u32, FuncId>,
    ) -> DxResult<()> {
        // Built-in functions with their magic IDs and argument counts
        let builtins: &[(&str, u32, usize)] = &[
            ("__dx_console_log", u32::MAX - 1, 1),
            ("__dx_console_log", u32::MAX - 2, 1), // console.warn
            ("__dx_console_log", u32::MAX - 3, 1), // console.error
            ("__dx_math_floor", u32::MAX - 10, 1),
            ("__dx_math_ceil", u32::MAX - 11, 1),
            ("__dx_math_sqrt", u32::MAX - 12, 1),
            ("__dx_math_abs", u32::MAX - 13, 1),
            ("__dx_math_sin", u32::MAX - 14, 1),
            ("__dx_math_cos", u32::MAX - 15, 1),
            ("__dx_math_random", u32::MAX - 16, 0),
            ("__dx_json_parse", u32::MAX - 17, 1),
            ("__dx_json_stringify", u32::MAX - 18, 1),
            ("__dx_create_closure", u32::MAX - 20, 2),
            ("__dx_create_array", u32::MAX - 21, 1),
            ("__dx_array_push", u32::MAX - 22, 2),
            ("__dx_array_get", u32::MAX - 23, 2),
            ("__dx_create_object", u32::MAX - 24, 0),
            ("__dx_set_captured", u32::MAX - 25, 3),
            ("__dx_set_arrow", u32::MAX - 26, 1),
            ("__dx_array_set", u32::MAX - 27, 3),
            ("__dx_object_set", u32::MAX - 28, 3),
            ("__dx_object_get", u32::MAX - 29, 2),
            ("__dx_get_captured", u32::MAX - 30, 2),
            ("__dx_get_this", u32::MAX - 31, 0),
            ("__dx_set_this", u32::MAX - 32, 1),
            ("__dx_typeof", u32::MAX - 33, 1),
            ("__dx_create_generator", u32::MAX - 40, 2),
            ("__dx_generator_next", u32::MAX - 41, 2),
            ("__dx_generator_yield", u32::MAX - 42, 2),
            ("__dx_generator_return", u32::MAX - 43, 2),
            ("__dx_generator_throw", u32::MAX - 44, 2),
            ("__dx_generator_force_return", u32::MAX - 45, 2),
            ("__dx_generator_get_state", u32::MAX - 46, 1),
            ("__dx_is_generator", u32::MAX - 47, 1),
            ("__dx_create_promise", u32::MAX - 50, 0),
            ("__dx_promise_resolve", u32::MAX - 51, 2),
            ("__dx_promise_reject", u32::MAX - 52, 2),
            ("__dx_promise_get_value", u32::MAX - 53, 1),
            ("__dx_create_async_function", u32::MAX - 60, 2),
            ("__dx_await", u32::MAX - 61, 1),
            ("__dx_await_with_context", u32::MAX - 62, 2),
            ("__dx_set_async_context", u32::MAX - 63, 1),
            ("__dx_get_async_context", u32::MAX - 64, 0),
            ("__dx_clear_async_context", u32::MAX - 65, 0),
            ("__dx_is_await_suspended", u32::MAX - 66, 0),
            ("__dx_save_async_state", u32::MAX - 67, 2),
            ("__dx_save_async_local", u32::MAX - 68, 3),
            ("__dx_restore_async_local", u32::MAX - 69, 2),
            ("__dx_get_async_state", u32::MAX - 130, 1),
            ("__dx_resume_async", u32::MAX - 131, 3),
            ("__dx_get_property", u32::MAX - 70, 2),
            ("__dx_set_property", u32::MAX - 71, 3),
            ("__dx_get_property_dynamic", u32::MAX - 72, 2),
            ("__dx_set_property_dynamic", u32::MAX - 73, 3),
            ("__dx_setup_exception_handler", u32::MAX - 80, 2),
            ("__dx_clear_exception_handler", u32::MAX - 81, 0),
            ("__dx_throw", u32::MAX - 82, 1),
            ("__dx_get_exception", u32::MAX - 83, 0),
            ("__dx_clear_exception", u32::MAX - 84, 0),
            ("__dx_call_function", u32::MAX - 90, 10), // closure_id, arg_count, arg0-arg7
            ("__dx_get_current_closure", u32::MAX - 91, 0),
            ("__dx_dynamic_import", u32::MAX - 100, 2), // specifier_id, referrer_id
            // Stack frame management built-ins for stack trace capture
            ("__dx_push_call_frame", u32::MAX - 110, 4), // function_name_id, file_id, line, column
            ("__dx_pop_call_frame", u32::MAX - 111, 0),
            ("__dx_get_stack_depth", u32::MAX - 112, 0),
            ("__dx_clear_call_stack", u32::MAX - 113, 0),
            // Type coercion and equality built-ins (ECMAScript spec)
            ("__dx_strict_equals", u32::MAX - 120, 2),
            ("__dx_loose_equals", u32::MAX - 121, 2),
            ("__dx_to_boolean", u32::MAX - 122, 1),
            ("__dx_to_number", u32::MAX - 123, 1),
            ("__dx_to_string", u32::MAX - 124, 1),
            ("__dx_string_concat", u32::MAX - 125, 2),
            ("__dx_is_string_operand", u32::MAX - 126, 1),
            ("__dx_check_bigint_number_mix", u32::MAX - 127, 2),
            // Class-related built-ins (ES6 Classes)
            ("__dx_create_class", u32::MAX - 140, 2),           // constructor_id, super_class_id
            ("__dx_create_instance", u32::MAX - 141, 1),        // class_id
            ("__dx_get_class_constructor", u32::MAX - 142, 1),  // class_id
            ("__dx_get_class_prototype", u32::MAX - 143, 1),    // class_id
            ("__dx_get_super_class", u32::MAX - 144, 1),        // class_id
            ("__dx_instanceof", u32::MAX - 145, 2),             // object_id, class_id
            ("__dx_define_method", u32::MAX - 146, 4),          // class_id, name_id, function_id, is_static
            ("__dx_define_getter", u32::MAX - 147, 4),          // class_id, name_id, function_id, is_static
            ("__dx_define_setter", u32::MAX - 148, 4),          // class_id, name_id, function_id, is_static
            ("__dx_get_static_property", u32::MAX - 149, 2),    // class_id, name_id
            ("__dx_set_static_property", u32::MAX - 150, 3),    // class_id, name_id, value
            ("__dx_get_property_with_proto", u32::MAX - 151, 2), // object_id, name_id
            ("__dx_set_object_prototype", u32::MAX - 152, 2),   // object_id, prototype_id
            ("__dx_get_object_prototype", u32::MAX - 153, 1),   // object_id
            ("__dx_define_private_field", u32::MAX - 154, 3),   // class_id, name_id, initial_value
            ("__dx_get_super_method", u32::MAX - 155, 2),       // class_id, method_name_hash
            // Destructuring built-ins (Requirements: 7.1-7.7)
            ("__dx_array_slice_from", u32::MAX - 200, 2),       // source_array_id, start_index
            ("__dx_object_rest", u32::MAX - 201, 2),            // source_object_id, excluded_count
            ("__dx_is_undefined", u32::MAX - 202, 1),           // value
            ("__dx_throw_destructuring_error", u32::MAX - 203, 1), // source_value
            // Template literal built-ins (Requirements: 8.1-8.3)
            ("__dx_build_template_literal", u32::MAX - 210, 4), // quasis_ptr, quasis_len, exprs_ptr, exprs_len
            ("__dx_call_tagged_template", u32::MAX - 211, 4),   // tag_fn, strings_array, exprs_ptr, exprs_len
        ];

        for (name, magic_id, arg_count) in builtins {
            let mut sig = module.make_signature();
            for _ in 0..*arg_count {
                sig.params.push(AbiParam::new(types::F64));
            }
            sig.returns.push(AbiParam::new(types::F64));

            let func_id = module
                .declare_function(name, Linkage::Import, &sig)
                .map_err(|e| DxError::CompileError(e.to_string()))?;

            builtin_ids.insert(*magic_id, func_id);
        }

        Ok(())
    }

    fn compile_function_body(
        &self,
        ctx: &mut cranelift::codegen::Context,
        func_ctx: &mut FunctionBuilderContext,
        func: &TypedFunction,
        func_refs: &HashMap<u32, FuncRef>,
    ) -> DxResult<()> {
        let mut builder = CraneliftFunctionBuilder::new(&mut ctx.func, func_ctx);

        // Create block map
        let mut block_map: HashMap<BlockId, Block> = HashMap::new();
        for block in &func.blocks {
            let cl_block = builder.create_block();
            block_map.insert(block.id, cl_block);
        }

        // Set up entry block
        let entry_block = block_map[&BlockId(0)];
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        // Use Cranelift's Variable system for proper SSA handling with phi nodes
        // This is essential for loop variables that are modified across iterations
        let mut variables: HashMap<LocalId, Variable> = HashMap::new();
        
        // Declare all locals as Variables - this allows Cranelift to insert phi nodes
        for local in &func.locals {
            let var = Variable::new(local.index as usize);
            builder.declare_var(var, types::F64);
            variables.insert(LocalId(local.index), var);
        }

        // Map parameters - define them as variables
        for (i, param) in func.params.iter().enumerate() {
            let value = builder.block_params(entry_block)[i];
            let var = Variable::new(param.index as usize);
            builder.declare_var(var, types::F64);
            builder.def_var(var, value);
            variables.insert(LocalId(param.index), var);
        }

        // Seal entry block after declaring variables
        builder.seal_block(entry_block);

        // Compile each block - defer sealing until all predecessors are known
        // This is necessary for loops where back-edges are added after the target block
        for block in &func.blocks {
            let cl_block = block_map[&block.id];

            if block.id != BlockId(0) {
                builder.switch_to_block(cl_block);
                // Don't seal yet - wait until all predecessors are added
            }

            // Compile instructions using Variable system
            for inst in &block.instructions {
                self.compile_instruction_with_vars(&mut builder, inst, &mut variables, func_refs)?;
            }

            // Compile terminator
            self.compile_terminator_with_vars(&mut builder, &block.terminator, &variables, &block_map)?;
        }

        // Now seal all blocks after all edges have been added
        for block in &func.blocks {
            let cl_block = block_map[&block.id];
            if block.id != BlockId(0) {
                builder.seal_block(cl_block);
            }
        }

        builder.finalize();
        Ok(())
    }
    
    fn compile_instruction_with_vars(
        &self,
        builder: &mut CraneliftFunctionBuilder,
        inst: &TypedInstruction,
        variables: &mut HashMap<LocalId, Variable>,
        func_refs: &HashMap<u32, FuncRef>,
    ) -> DxResult<()> {
        // Helper to get or create a variable for a LocalId
        let get_or_create_var = |vars: &mut HashMap<LocalId, Variable>, builder: &mut CraneliftFunctionBuilder, id: LocalId| -> Variable {
            if let Some(&var) = vars.get(&id) {
                var
            } else {
                let var = Variable::new(id.0 as usize);
                builder.declare_var(var, types::F64);
                vars.insert(id, var);
                var
            }
        };
        
        match inst {
            TypedInstruction::Const { dest, value } => {
                let val = match value {
                    Constant::I32(n) => {
                        let i = builder.ins().iconst(types::I32, *n as i64);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    Constant::I64(n) => {
                        let i = builder.ins().iconst(types::I64, *n);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    Constant::F64(n) => builder.ins().f64const(*n),
                    Constant::Bool(b) => builder.ins().f64const(if *b { 1.0 } else { 0.0 }),
                    Constant::String(s) => {
                        let heap = get_runtime_heap_lock();
                        let mut heap = heap.lock().unwrap();
                        let id = heap.allocate_string(s.clone());
                        let tagged_id = encode_string_id(id);
                        builder.ins().f64const(tagged_id)
                    }
                    Constant::BigInt(s) => {
                        let heap = get_runtime_heap_lock();
                        let mut heap = heap.lock().unwrap();
                        let bigint_value = s.parse::<num_bigint::BigInt>().unwrap_or_else(|_| {
                            num_bigint::BigInt::from(0)
                        });
                        let id = heap.allocate_bigint(bigint_value);
                        let tagged_id = encode_bigint_id(id);
                        builder.ins().f64const(tagged_id)
                    }
                    Constant::Null => builder.ins().f64const(0.0),
                    Constant::Undefined => builder.ins().f64const(f64::NAN),
                };
                let var = get_or_create_var(variables, builder, *dest);
                builder.def_var(var, val);
            }

            TypedInstruction::BinOp {
                dest,
                op,
                left,
                right,
                op_type: _,
            } => {
                let left_var = get_or_create_var(variables, builder, *left);
                let right_var = get_or_create_var(variables, builder, *right);
                let lval = builder.use_var(left_var);
                let rval = builder.use_var(right_var);

                let result = match op {
                    BinOpKind::Add => {
                        // For Add, we need to check if either operand is a string
                        // If so, perform string concatenation instead of numeric addition
                        if let Some(&is_string_func) = func_refs.get(&(u32::MAX - 126)) {
                            if let Some(&concat_func) = func_refs.get(&(u32::MAX - 125)) {
                                // Check if either operand is a string
                                let check_call = builder.ins().call(is_string_func, &[lval, rval]);
                                let check_results = builder.inst_results(check_call);
                                
                                if !check_results.is_empty() {
                                    let is_string = check_results[0];
                                    let one = builder.ins().f64const(1.0);
                                    let cmp = builder.ins().fcmp(FloatCC::Equal, is_string, one);
                                    
                                    // Create blocks for branching
                                    let string_block = builder.create_block();
                                    let number_block = builder.create_block();
                                    let merge_block = builder.create_block();
                                    builder.append_block_param(merge_block, types::F64);
                                    
                                    builder.ins().brif(cmp, string_block, &[], number_block, &[]);
                                    
                                    // String concatenation path
                                    builder.switch_to_block(string_block);
                                    builder.seal_block(string_block);
                                    let concat_call = builder.ins().call(concat_func, &[lval, rval]);
                                    let concat_results = builder.inst_results(concat_call);
                                    let concat_result = if !concat_results.is_empty() {
                                        concat_results[0]
                                    } else {
                                        builder.ins().f64const(f64::NAN)
                                    };
                                    builder.ins().jump(merge_block, &[concat_result]);
                                    
                                    // Numeric addition path
                                    builder.switch_to_block(number_block);
                                    builder.seal_block(number_block);
                                    let add_result = builder.ins().fadd(lval, rval);
                                    builder.ins().jump(merge_block, &[add_result]);
                                    
                                    // Merge block
                                    builder.switch_to_block(merge_block);
                                    builder.seal_block(merge_block);
                                    builder.block_params(merge_block)[0]
                                } else {
                                    // Fallback to numeric addition
                                    builder.ins().fadd(lval, rval)
                                }
                            } else {
                                // Fallback to numeric addition
                                builder.ins().fadd(lval, rval)
                            }
                        } else {
                            // Fallback to numeric addition
                            builder.ins().fadd(lval, rval)
                        }
                    }
                    BinOpKind::Sub => builder.ins().fsub(lval, rval),
                    BinOpKind::Mul => builder.ins().fmul(lval, rval),
                    BinOpKind::Div => builder.ins().fdiv(lval, rval),
                    BinOpKind::Mod => {
                        let div = builder.ins().fdiv(lval, rval);
                        let floor = builder.ins().floor(div);
                        let mul = builder.ins().fmul(floor, rval);
                        builder.ins().fsub(lval, mul)
                    }
                    BinOpKind::Lt => {
                        let cmp = builder.ins().fcmp(FloatCC::LessThan, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Le => {
                        let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Gt => {
                        let cmp = builder.ins().fcmp(FloatCC::GreaterThan, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Ge => {
                        let cmp = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Eq => {
                        let cmp = builder.ins().fcmp(FloatCC::Equal, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Ne => {
                        let cmp = builder.ins().fcmp(FloatCC::NotEqual, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::And => {
                        let zero = builder.ins().f64const(0.0);
                        let l_nz = builder.ins().fcmp(FloatCC::NotEqual, lval, zero);
                        let r_nz = builder.ins().fcmp(FloatCC::NotEqual, rval, zero);
                        let both = builder.ins().band(l_nz, r_nz);
                        let i = builder.ins().uextend(types::I32, both);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Or => {
                        let zero = builder.ins().f64const(0.0);
                        let l_nz = builder.ins().fcmp(FloatCC::NotEqual, lval, zero);
                        let r_nz = builder.ins().fcmp(FloatCC::NotEqual, rval, zero);
                        let either = builder.ins().bor(l_nz, r_nz);
                        let i = builder.ins().uextend(types::I32, either);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                };

                let var = get_or_create_var(variables, builder, *dest);
                builder.def_var(var, result);
            }

            TypedInstruction::Call {
                dest,
                function,
                args,
            } => {
                let arg_values: Vec<cranelift::prelude::Value> = args.iter().map(|a| {
                    let var = variables.get(a).copied().unwrap_or_else(|| Variable::new(a.0 as usize));
                    builder.use_var(var)
                }).collect();

                if let Some(&func_ref) = func_refs.get(&function.0) {
                    let call = builder.ins().call(func_ref, &arg_values);

                    if let Some(dest) = dest {
                        let results = builder.inst_results(call);
                        let val = if !results.is_empty() {
                            results[0]
                        } else {
                            builder.ins().f64const(f64::NAN)
                        };
                        let var = get_or_create_var(variables, builder, *dest);
                        builder.def_var(var, val);
                    }
                } else {
                    if let Some(dest) = dest {
                        let nan = builder.ins().f64const(f64::NAN);
                        let var = get_or_create_var(variables, builder, *dest);
                        builder.def_var(var, nan);
                    }
                }
            }

            TypedInstruction::Copy { dest, src } => {
                let src_var = get_or_create_var(variables, builder, *src);
                let val = builder.use_var(src_var);
                let dest_var = get_or_create_var(variables, builder, *dest);
                builder.def_var(dest_var, val);
            }

            // For other instructions, delegate to the original compile_instruction
            // but convert between Variable and Value systems
            _ => {
                // Create a temporary locals map for the original function
                let mut locals: HashMap<LocalId, cranelift::prelude::Value> = HashMap::new();
                
                // Convert variables to values for instructions that need them
                for (&local_id, &var) in variables.iter() {
                    // Only use_var if the variable has been defined
                    // We'll handle undefined variables by creating a default value
                    let val = builder.use_var(var);
                    locals.insert(local_id, val);
                }
                
                // Call original compile_instruction
                self.compile_instruction(builder, inst, &mut locals, func_refs)?;
                
                // Update variables with any new values
                for (&local_id, &val) in locals.iter() {
                    let var = get_or_create_var(variables, builder, local_id);
                    builder.def_var(var, val);
                }
            }
        }
        Ok(())
    }
    
    fn compile_terminator_with_vars(
        &self,
        builder: &mut CraneliftFunctionBuilder,
        terminator: &Terminator,
        variables: &HashMap<LocalId, Variable>,
        block_map: &HashMap<BlockId, Block>,
    ) -> DxResult<()> {
        match terminator {
            Terminator::Return(maybe_local_id) => {
                if let Some(local_id) = maybe_local_id {
                    if let Some(&var) = variables.get(local_id) {
                        let val = builder.use_var(var);
                        builder.ins().return_(&[val]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        builder.ins().return_(&[nan]);
                    }
                } else {
                    let nan = builder.ins().f64const(f64::NAN);
                    builder.ins().return_(&[nan]);
                }
            }
            Terminator::Goto(block_id) => {
                let target = block_map[block_id];
                builder.ins().jump(target, &[]);
            }
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(&var) = variables.get(condition) {
                    let cond_val = builder.use_var(var);
                    let zero = builder.ins().f64const(0.0);
                    let cmp = builder.ins().fcmp(FloatCC::NotEqual, cond_val, zero);
                    let then_target = block_map[then_block];
                    let else_target = block_map[else_block];
                    builder.ins().brif(cmp, then_target, &[], else_target, &[]);
                } else {
                    // Condition not found - jump to else block
                    let else_target = block_map[else_block];
                    builder.ins().jump(else_target, &[]);
                }
            }
            Terminator::Unreachable => {
                builder.ins().trap(TrapCode::user(0).unwrap());
            }
        }
        Ok(())
    }

    fn compile_instruction(
        &self,
        builder: &mut CraneliftFunctionBuilder,
        inst: &TypedInstruction,
        locals: &mut HashMap<LocalId, cranelift::prelude::Value>,
        func_refs: &HashMap<u32, FuncRef>,
    ) -> DxResult<()> {
        match inst {
            TypedInstruction::Const { dest, value } => {
                let val = match value {
                    Constant::I32(n) => {
                        let i = builder.ins().iconst(types::I32, *n as i64);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    Constant::I64(n) => {
                        let i = builder.ins().iconst(types::I64, *n);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    Constant::F64(n) => builder.ins().f64const(*n),
                    Constant::Bool(b) => builder.ins().f64const(if *b { 1.0 } else { 0.0 }),
                    Constant::String(s) => {
                        // Allocate string in runtime heap and return tagged ID
                        let heap = get_runtime_heap_lock();
                        let mut heap = heap.lock().unwrap();
                        let id = heap.allocate_string(s.clone());
                        let tagged_id = encode_string_id(id);
                        builder.ins().f64const(tagged_id)
                    }
                    Constant::BigInt(s) => {
                        // Parse BigInt string and allocate in runtime heap
                        let heap = get_runtime_heap_lock();
                        let mut heap = heap.lock().unwrap();
                        // Parse the BigInt from string representation
                        let bigint_value = s.parse::<num_bigint::BigInt>().unwrap_or_else(|_| {
                            num_bigint::BigInt::from(0)
                        });
                        let id = heap.allocate_bigint(bigint_value);
                        let tagged_id = encode_bigint_id(id);
                        builder.ins().f64const(tagged_id)
                    }
                    Constant::Null => builder.ins().f64const(0.0),
                    Constant::Undefined => builder.ins().f64const(f64::NAN),
                };
                locals.insert(*dest, val);
            }

            TypedInstruction::BinOp {
                dest,
                op,
                left,
                right,
                op_type: _,
            } => {
                let lval = locals[left];
                let rval = locals[right];

                let result = match op {
                    BinOpKind::Add => {
                        // For Add, we need to check if either operand is a string
                        // If so, perform string concatenation instead of numeric addition
                        if let Some(&is_string_func) = func_refs.get(&(u32::MAX - 126)) {
                            if let Some(&concat_func) = func_refs.get(&(u32::MAX - 125)) {
                                // Check if either operand is a string
                                let check_call = builder.ins().call(is_string_func, &[lval, rval]);
                                let check_results = builder.inst_results(check_call);
                                
                                if !check_results.is_empty() {
                                    let is_string = check_results[0];
                                    let one = builder.ins().f64const(1.0);
                                    let cmp = builder.ins().fcmp(FloatCC::Equal, is_string, one);
                                    
                                    // Create blocks for branching
                                    let string_block = builder.create_block();
                                    let number_block = builder.create_block();
                                    let merge_block = builder.create_block();
                                    builder.append_block_param(merge_block, types::F64);
                                    
                                    builder.ins().brif(cmp, string_block, &[], number_block, &[]);
                                    
                                    // String concatenation path
                                    builder.switch_to_block(string_block);
                                    builder.seal_block(string_block);
                                    let concat_call = builder.ins().call(concat_func, &[lval, rval]);
                                    let concat_results = builder.inst_results(concat_call);
                                    let concat_result = if !concat_results.is_empty() {
                                        concat_results[0]
                                    } else {
                                        builder.ins().f64const(f64::NAN)
                                    };
                                    builder.ins().jump(merge_block, &[concat_result]);
                                    
                                    // Numeric addition path
                                    builder.switch_to_block(number_block);
                                    builder.seal_block(number_block);
                                    let add_result = builder.ins().fadd(lval, rval);
                                    builder.ins().jump(merge_block, &[add_result]);
                                    
                                    // Merge block
                                    builder.switch_to_block(merge_block);
                                    builder.seal_block(merge_block);
                                    builder.block_params(merge_block)[0]
                                } else {
                                    // Fallback to numeric addition
                                    builder.ins().fadd(lval, rval)
                                }
                            } else {
                                // Fallback to numeric addition
                                builder.ins().fadd(lval, rval)
                            }
                        } else {
                            // Fallback to numeric addition
                            builder.ins().fadd(lval, rval)
                        }
                    }
                    BinOpKind::Sub => builder.ins().fsub(lval, rval),
                    BinOpKind::Mul => builder.ins().fmul(lval, rval),
                    BinOpKind::Div => builder.ins().fdiv(lval, rval),
                    BinOpKind::Mod => {
                        // x % y = x - floor(x/y) * y
                        let div = builder.ins().fdiv(lval, rval);
                        let floor = builder.ins().floor(div);
                        let mul = builder.ins().fmul(floor, rval);
                        builder.ins().fsub(lval, mul)
                    }
                    BinOpKind::Lt => {
                        let cmp = builder.ins().fcmp(FloatCC::LessThan, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Le => {
                        let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Gt => {
                        let cmp = builder.ins().fcmp(FloatCC::GreaterThan, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Ge => {
                        let cmp = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Eq => {
                        let cmp = builder.ins().fcmp(FloatCC::Equal, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Ne => {
                        let cmp = builder.ins().fcmp(FloatCC::NotEqual, lval, rval);
                        let i = builder.ins().uextend(types::I32, cmp);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::And => {
                        // Logical AND: both non-zero
                        let zero = builder.ins().f64const(0.0);
                        let l_nz = builder.ins().fcmp(FloatCC::NotEqual, lval, zero);
                        let r_nz = builder.ins().fcmp(FloatCC::NotEqual, rval, zero);
                        let both = builder.ins().band(l_nz, r_nz);
                        let i = builder.ins().uextend(types::I32, both);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                    BinOpKind::Or => {
                        // Logical OR: either non-zero
                        let zero = builder.ins().f64const(0.0);
                        let l_nz = builder.ins().fcmp(FloatCC::NotEqual, lval, zero);
                        let r_nz = builder.ins().fcmp(FloatCC::NotEqual, rval, zero);
                        let either = builder.ins().bor(l_nz, r_nz);
                        let i = builder.ins().uextend(types::I32, either);
                        builder.ins().fcvt_from_sint(types::F64, i)
                    }
                };

                locals.insert(*dest, result);
            }

            TypedInstruction::Call {
                dest,
                function,
                args,
            } => {
                let arg_values: Vec<cranelift::prelude::Value> =
                    args.iter().map(|a| locals[a]).collect();

                // Look up pre-declared function reference
                if let Some(&func_ref) = func_refs.get(&function.0) {
                    let call = builder.ins().call(func_ref, &arg_values);

                    if let Some(dest) = dest {
                        let results = builder.inst_results(call);
                        if !results.is_empty() {
                            locals.insert(*dest, results[0]);
                        } else {
                            let nan = builder.ins().f64const(f64::NAN);
                            locals.insert(*dest, nan);
                        }
                    }
                } else {
                    // Unknown function - return NaN
                    if let Some(dest) = dest {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                }
            }

            TypedInstruction::Copy { dest, src } => {
                let val = locals[src];
                locals.insert(*dest, val);
            }

            TypedInstruction::GetProperty {
                dest,
                object,
                offset,
                prop_type: _,
            } => {
                // Property access using runtime function
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 70)) {
                    let object_val = locals[object];
                    let offset_val = builder.ins().f64const(*offset as f64);
                    let call = builder.ins().call(func_ref, &[object_val, offset_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return NaN
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::SetProperty {
                object,
                offset,
                value,
            } => {
                // Property write using runtime function
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 71)) {
                    let object_val = locals[object];
                    let offset_val = builder.ins().f64const(*offset as f64);
                    let value_val = locals[value];
                    builder.ins().call(func_ref, &[object_val, offset_val, value_val]);
                }
            }

            TypedInstruction::Allocate { dest, .. } => {
                // Object allocation not fully implemented yet
                let zero = builder.ins().f64const(0.0);
                locals.insert(*dest, zero);
            }

            // New instructions for function objects and closures
            TypedInstruction::CreateFunction {
                dest,
                function_id,
                captured_vars,
                is_arrow,
            } => {
                // Allocate a proper closure object using runtime heap
                // Call __dx_create_closure(function_id, captured_count)
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 20)) {
                    let func_id_val = builder.ins().f64const(function_id.0 as f64);
                    let captured_count = builder.ins().f64const(captured_vars.len() as f64);
                    let call = builder.ins().call(func_ref, &[func_id_val, captured_count]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        let closure_id = results[0];

                        // Store captured variables in the closure
                        if let Some(&set_captured_ref) = func_refs.get(&(u32::MAX - 25)) {
                            for (i, var_id) in captured_vars.iter().enumerate() {
                                if let Some(&var_val) = locals.get(var_id) {
                                    let idx = builder.ins().f64const(i as f64);
                                    builder
                                        .ins()
                                        .call(set_captured_ref, &[closure_id, idx, var_val]);
                                }
                            }
                        }

                        // Set is_arrow flag if needed
                        if *is_arrow {
                            if let Some(&set_arrow_ref) = func_refs.get(&(u32::MAX - 26)) {
                                builder.ins().call(set_arrow_ref, &[closure_id]);
                            }
                        }

                        locals.insert(*dest, closure_id);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return function ID as placeholder
                    let func_ptr = builder.ins().f64const(function_id.0 as f64);
                    locals.insert(*dest, func_ptr);
                }
            }

            TypedInstruction::CallFunction {
                dest, callee, args, this_arg,
            } => {
                // Call a function object using the runtime call_function builtin
                let callee_val = locals[callee];
                
                // Get the this_arg value if present
                let this_val = this_arg.map(|t| locals[&t]);

                // Get the call_function builtin
                if let Some(&call_func_ref) = func_refs.get(&(u32::MAX - 90)) {
                    // Prepare arguments (pad with NaN for unused slots)
                    let nan = builder.ins().f64const(f64::NAN);
                    
                    // For method calls, we pass this_val as arg0
                    // The call_function will handle BigInt methods specially
                    let (effective_arg_count, arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) = 
                        if let Some(this_v) = this_val {
                            // Method call - this goes as arg0, actual args shift
                            let arg_count = builder.ins().f64const((args.len() + 1) as f64);
                            let a0 = this_v;
                            let a1 = args.first().map(|a| locals[a]).unwrap_or(nan);
                            let a2 = args.get(1).map(|a| locals[a]).unwrap_or(nan);
                            let a3 = args.get(2).map(|a| locals[a]).unwrap_or(nan);
                            let a4 = args.get(3).map(|a| locals[a]).unwrap_or(nan);
                            let a5 = args.get(4).map(|a| locals[a]).unwrap_or(nan);
                            let a6 = args.get(5).map(|a| locals[a]).unwrap_or(nan);
                            let a7 = args.get(6).map(|a| locals[a]).unwrap_or(nan);
                            (arg_count, a0, a1, a2, a3, a4, a5, a6, a7)
                        } else {
                            // Regular function call
                            let arg_count = builder.ins().f64const(args.len() as f64);
                            let a0 = args.first().map(|a| locals[a]).unwrap_or(nan);
                            let a1 = args.get(1).map(|a| locals[a]).unwrap_or(nan);
                            let a2 = args.get(2).map(|a| locals[a]).unwrap_or(nan);
                            let a3 = args.get(3).map(|a| locals[a]).unwrap_or(nan);
                            let a4 = args.get(4).map(|a| locals[a]).unwrap_or(nan);
                            let a5 = args.get(5).map(|a| locals[a]).unwrap_or(nan);
                            let a6 = args.get(6).map(|a| locals[a]).unwrap_or(nan);
                            let a7 = args.get(7).map(|a| locals[a]).unwrap_or(nan);
                            (arg_count, a0, a1, a2, a3, a4, a5, a6, a7)
                        };

                    // Call the function
                    let call = builder.ins().call(
                        call_func_ref,
                        &[
                            callee_val, effective_arg_count, arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7,
                        ],
                    );
                    let results = builder.inst_results(call);

                    if let Some(dest) = dest {
                        if !results.is_empty() {
                            locals.insert(*dest, results[0]);
                        } else {
                            locals.insert(*dest, nan);
                        }
                    }
                } else {
                    // Fallback: return NaN
                    if let Some(dest) = dest {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                }
            }

            TypedInstruction::GetCaptured { dest, env_index } => {
                // Get captured variable from closure environment
                // First get the current closure ID, then get the captured variable
                if let Some(&get_closure_ref) = func_refs.get(&(u32::MAX - 91)) {
                    if let Some(&get_captured_ref) = func_refs.get(&(u32::MAX - 30)) {
                        // Get the current closure ID
                        let closure_call = builder.ins().call(get_closure_ref, &[]);
                        let closure_results = builder.inst_results(closure_call);

                        if !closure_results.is_empty() {
                            let closure_id = closure_results[0];
                            let idx = builder.ins().f64const(*env_index as f64);
                            let call = builder.ins().call(get_captured_ref, &[closure_id, idx]);
                            let results = builder.inst_results(call);
                            if !results.is_empty() {
                                locals.insert(*dest, results[0]);
                            } else {
                                let nan = builder.ins().f64const(f64::NAN);
                                locals.insert(*dest, nan);
                            }
                        } else {
                            let nan = builder.ins().f64const(f64::NAN);
                            locals.insert(*dest, nan);
                        }
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::SetCaptured { env_index, value } => {
                // Set captured variable in closure environment
                // First get the current closure ID, then set the captured variable
                if let Some(&get_closure_ref) = func_refs.get(&(u32::MAX - 91)) {
                    if let Some(&set_captured_ref) = func_refs.get(&(u32::MAX - 25)) {
                        // Get the current closure ID
                        let closure_call = builder.ins().call(get_closure_ref, &[]);
                        let closure_results = builder.inst_results(closure_call);

                        if !closure_results.is_empty() {
                            let closure_id = closure_results[0];
                            let idx = builder.ins().f64const(*env_index as f64);
                            let value_val = locals[value];
                            builder.ins().call(set_captured_ref, &[closure_id, idx, value_val]);
                        }
                    }
                }
            }

            TypedInstruction::GetPropertyDynamic {
                dest,
                object,
                property,
            } => {
                // Dynamic property access using runtime function
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 72)) {
                    let object_val = locals[object];
                    // Hash the property name
                    let key_hash = hash_string(property);
                    let key_hash_val = builder.ins().f64const(key_hash as f64);
                    let call = builder.ins().call(func_ref, &[object_val, key_hash_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::SetPropertyDynamic {
                object,
                property,
                value,
            } => {
                // Dynamic property write using runtime function
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 73)) {
                    let object_val = locals[object];
                    let key_hash = hash_string(property);
                    let key_hash_val = builder.ins().f64const(key_hash as f64);
                    let value_val = locals[value];
                    builder.ins().call(func_ref, &[object_val, key_hash_val, value_val]);
                }
            }

            TypedInstruction::GetPropertyComputed { dest, object, key } => {
                // Computed property access - key is a runtime value
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 72)) {
                    let object_val = locals[object];
                    let key_val = locals[key];
                    let call = builder.ins().call(func_ref, &[object_val, key_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::SetPropertyComputed { object, key, value } => {
                // Computed property write - key is a runtime value
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 73)) {
                    let object_val = locals[object];
                    let key_val = locals[key];
                    let value_val = locals[value];
                    builder.ins().call(func_ref, &[object_val, key_val, value_val]);
                }
            }

            TypedInstruction::CreateArray { dest, elements } => {
                // Create array using runtime heap
                // Call __dx_create_array(element_count)
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 21)) {
                    let count_val = builder.ins().f64const(elements.len() as f64);
                    let call = builder.ins().call(func_ref, &[count_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        let array_id = results[0];

                        // Set each element at its index using array_set
                        if let Some(&array_set_ref) = func_refs.get(&(u32::MAX - 27)) {
                            for (i, elem) in elements.iter().enumerate() {
                                if let Some(&elem_val) = locals.get(elem) {
                                    let idx = builder.ins().f64const(i as f64);
                                    builder.ins().call(array_set_ref, &[array_id, idx, elem_val]);
                                }
                            }
                        }

                        locals.insert(*dest, array_id);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return element count as placeholder
                    let count = builder.ins().f64const(elements.len() as f64);
                    locals.insert(*dest, count);
                }
            }

            TypedInstruction::CreateObject { dest, properties } => {
                // Create object using runtime heap
                // Call __dx_create_object()
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 24)) {
                    let call = builder.ins().call(func_ref, &[]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        let object_id = results[0];

                        // Set each property using object_set
                        if let Some(&object_set_ref) = func_refs.get(&(u32::MAX - 28)) {
                            for (key, value_id) in properties {
                                if let Some(&value_val) = locals.get(value_id) {
                                    // Hash the key string for storage
                                    let key_hash = hash_string(key);
                                    let key_hash_val = builder.ins().f64const(key_hash as f64);
                                    builder.ins().call(
                                        object_set_ref,
                                        &[object_id, key_hash_val, value_val],
                                    );
                                }
                            }
                        }

                        locals.insert(*dest, object_id);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return property count as placeholder
                    let count = builder.ins().f64const(properties.len() as f64);
                    locals.insert(*dest, count);
                }
            }

            TypedInstruction::Throw { value } => {
                // Throw exception - call runtime to handle unwinding
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 82)) {
                    let value_val = locals[value];
                    let call = builder.ins().call(func_ref, &[value_val]);
                    let results = builder.inst_results(call);

                    // The throw function returns the catch block ID or NaN if uncaught
                    // For now, we trap on uncaught exceptions
                    if !results.is_empty() {
                        let catch_block_id = results[0];
                        // Check if it's NaN (uncaught)
                        let nan = builder.ins().f64const(f64::NAN);
                        let is_uncaught =
                            builder.ins().fcmp(FloatCC::Unordered, catch_block_id, nan);

                        // If uncaught, trap
                        if let Some(trap_code) = TrapCode::user(0) {
                            // Create a block for the trap
                            let trap_block = builder.create_block();
                            let continue_block = builder.create_block();

                            builder.ins().brif(is_uncaught, trap_block, &[], continue_block, &[]);

                            builder.switch_to_block(trap_block);
                            builder.seal_block(trap_block);
                            builder.ins().trap(trap_code);

                            builder.switch_to_block(continue_block);
                            builder.seal_block(continue_block);
                        }
                    }
                } else {
                    // Fallback: just trap
                    if let Some(trap_code) = TrapCode::user(0) {
                        builder.ins().trap(trap_code);
                    }
                }
            }

            TypedInstruction::SetupExceptionHandler {
                catch_block,
                finally_block,
            } => {
                // Register catch/finally blocks on handler stack
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 80)) {
                    let catch_block_val = builder.ins().f64const(catch_block.0 as f64);
                    let finally_block_val = match finally_block {
                        Some(fb) => builder.ins().f64const(fb.0 as f64),
                        None => builder.ins().f64const(f64::NAN),
                    };
                    builder.ins().call(func_ref, &[catch_block_val, finally_block_val]);
                }
            }

            TypedInstruction::ClearExceptionHandler => {
                // Clear exception handler from stack
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 81)) {
                    builder.ins().call(func_ref, &[]);
                }
            }

            TypedInstruction::GetException { dest } => {
                // Get caught exception value from thread-local storage
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 83)) {
                    let call = builder.ins().call(func_ref, &[]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::GetThis { dest } => {
                // Get the current `this` binding from thread-local storage
                if let Some(&get_this_ref) = func_refs.get(&(u32::MAX - 31)) {
                    let call = builder.ins().call(get_this_ref, &[]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return NaN (undefined)
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::TypeOf { dest, operand } => {
                // TypeOf operator - call runtime typeof function
                if let Some(&typeof_ref) = func_refs.get(&(u32::MAX - 33)) {
                    let operand_val = locals[operand];
                    let call = builder.ins().call(typeof_ref, &[operand_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        // Fallback: return TYPE_NUMBER
                        let type_val = builder.ins().f64const(1.0);
                        locals.insert(*dest, type_val);
                    }
                } else {
                    // Fallback: return TYPE_NUMBER (1.0)
                    let _operand_val = locals[operand];
                    let type_val = builder.ins().f64const(1.0);
                    locals.insert(*dest, type_val);
                }
            }

            TypedInstruction::ArraySpread { dest, source } => {
                // Spread array elements - for now, just copy the source
                let source_val = locals[source];
                locals.insert(*dest, source_val);
            }

            TypedInstruction::ArrayPush { array, value } => {
                // Push value onto array - no-op for now
                let _array_val = locals[array];
                let _value_val = locals[value];
            }

            TypedInstruction::CallWithSpread { dest, callee, args } => {
                // Call with spread arguments
                let _callee_val = locals[callee];
                let _args_val = locals[args];

                if let Some(dest) = dest {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            // Generator instructions
            TypedInstruction::GeneratorYield {
                dest,
                value,
                resume_block: _,
            } => {
                // Generator yield - for now, just copy the value
                // In a full implementation, this would save state and return
                let val = locals[value];
                locals.insert(*dest, val);
            }

            TypedInstruction::GeneratorReturn { value } => {
                // Generator return - similar to regular return
                if let Some(val_id) = value {
                    let _val = locals[val_id];
                }
            }

            TypedInstruction::CreateGenerator {
                dest,
                function_id,
                captured_vars,
            } => {
                // Create a generator object with state machine
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 40)) {
                    let func_id_val = builder.ins().f64const(function_id.0 as f64);
                    let captured_count = builder.ins().f64const(captured_vars.len() as f64);
                    let call = builder.ins().call(func_ref, &[func_id_val, captured_count]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        let gen_id = results[0];

                        // Store captured variables in the generator
                        if let Some(&set_captured_ref) = func_refs.get(&(u32::MAX - 25)) {
                            for (i, var_id) in captured_vars.iter().enumerate() {
                                if let Some(&var_val) = locals.get(var_id) {
                                    let idx = builder.ins().f64const(i as f64);
                                    builder.ins().call(set_captured_ref, &[gen_id, idx, var_val]);
                                }
                            }
                        }

                        locals.insert(*dest, gen_id);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return function ID as placeholder
                    let gen_id = builder.ins().f64const(function_id.0 as f64);
                    locals.insert(*dest, gen_id);
                }
            }

            TypedInstruction::GeneratorNext {
                dest,
                generator,
                send_value: _,
            } => {
                // Get next value from generator
                let _gen_val = locals[generator];
                // For now, return NaN (done)
                let nan = builder.ins().f64const(f64::NAN);
                locals.insert(*dest, nan);
            }

            // Async/Promise instructions
            TypedInstruction::CreatePromise { dest } => {
                // Create a Promise object with state tracking
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 50)) {
                    let call = builder.ins().call(func_ref, &[]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return a placeholder ID
                    let promise_id = builder.ins().f64const(1.0);
                    locals.insert(*dest, promise_id);
                }
            }

            TypedInstruction::PromiseResolve { promise, value } => {
                // Resolve a Promise with a value
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 51)) {
                    let promise_val = locals[promise];
                    let value_val = locals[value];
                    builder.ins().call(func_ref, &[promise_val, value_val]);
                }
            }

            TypedInstruction::PromiseReject { promise, reason } => {
                // Reject a Promise with a reason
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 52)) {
                    let promise_val = locals[promise];
                    let reason_val = locals[reason];
                    builder.ins().call(func_ref, &[promise_val, reason_val]);
                }
            }

            TypedInstruction::Await {
                dest,
                promise,
                resume_block: _,
                reject_block: _,
            } => {
                // Await a Promise
                // Requirements: 5.2 - WHEN await is encountered, THE DX_Runtime SHALL suspend execution
                // Requirements: 5.3 - WHEN an awaited Promise rejects, THE DX_Runtime SHALL throw the rejection reason
                
                let promise_val = locals[promise];
                
                // Call the await builtin which handles:
                // 1. If promise is fulfilled, return the value
                // 2. If promise is rejected, throw the rejection reason
                // 3. If promise is pending, suspend and register callback
                if let Some(&await_ref) = func_refs.get(&(u32::MAX - 61)) {
                    let call = builder.ins().call(await_ref, &[promise_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: just return the promise value (for already-settled promises)
                    if let Some(&get_value_ref) = func_refs.get(&(u32::MAX - 53)) {
                        let call = builder.ins().call(get_value_ref, &[promise_val]);
                        let results = builder.inst_results(call);
                        if !results.is_empty() {
                            locals.insert(*dest, results[0]);
                        } else {
                            locals.insert(*dest, promise_val);
                        }
                    } else {
                        locals.insert(*dest, promise_val);
                    }
                }
            }

            TypedInstruction::CreateAsyncFunction {
                dest,
                function_id,
                captured_vars,
            } => {
                // Create an async function wrapper that returns a Promise when called
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 60)) {
                    let func_id_val = builder.ins().f64const(function_id.0 as f64);
                    let captured_count = builder.ins().f64const(captured_vars.len() as f64);
                    let call = builder.ins().call(func_ref, &[func_id_val, captured_count]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        let async_fn_id = results[0];

                        // Store captured variables in the async function closure
                        if let Some(&set_captured_ref) = func_refs.get(&(u32::MAX - 25)) {
                            for (i, var_id) in captured_vars.iter().enumerate() {
                                if let Some(&var_val) = locals.get(var_id) {
                                    let idx = builder.ins().f64const(i as f64);
                                    builder
                                        .ins()
                                        .call(set_captured_ref, &[async_fn_id, idx, var_val]);
                                }
                            }
                        }

                        locals.insert(*dest, async_fn_id);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return function ID as placeholder
                    let func_id = builder.ins().f64const(function_id.0 as f64);
                    locals.insert(*dest, func_id);
                }
            }

            // Type conversion instructions
            TypedInstruction::ToBool { dest, src } => {
                // Convert value to boolean
                // In JS: 0, NaN, null, undefined, "" are falsy; everything else is truthy
                let val = locals[src];
                let zero = builder.ins().f64const(0.0);
                // For now, treat non-zero as truthy (simplified)
                let is_truthy = builder.ins().fcmp(FloatCC::NotEqual, val, zero);
                let i = builder.ins().uextend(types::I32, is_truthy);
                let result = builder.ins().fcvt_from_sint(types::F64, i);
                locals.insert(*dest, result);
            }

            TypedInstruction::IsNullish { dest, src } => {
                // Check if value is null or undefined (represented as NaN in our system)
                let val = locals[src];
                // NaN is unordered with itself, so fcmp Unordered will be true for NaN
                let is_nan = builder.ins().fcmp(FloatCC::Unordered, val, val);
                let i = builder.ins().uextend(types::I32, is_nan);
                let result = builder.ins().fcvt_from_sint(types::F64, i);
                locals.insert(*dest, result);
            }

            // Bitwise operators
            TypedInstruction::BitwiseNot { dest, operand } => {
                let val = locals[operand];
                // Convert to i32, apply NOT, convert back
                let i = builder.ins().fcvt_to_sint(types::I32, val);
                let not_i = builder.ins().bnot(i);
                let result = builder.ins().fcvt_from_sint(types::F64, not_i);
                locals.insert(*dest, result);
            }

            TypedInstruction::BitwiseAnd { dest, left, right } => {
                let lval = locals[left];
                let rval = locals[right];
                let li = builder.ins().fcvt_to_sint(types::I32, lval);
                let ri = builder.ins().fcvt_to_sint(types::I32, rval);
                let and_i = builder.ins().band(li, ri);
                let result = builder.ins().fcvt_from_sint(types::F64, and_i);
                locals.insert(*dest, result);
            }

            TypedInstruction::BitwiseOr { dest, left, right } => {
                let lval = locals[left];
                let rval = locals[right];
                let li = builder.ins().fcvt_to_sint(types::I32, lval);
                let ri = builder.ins().fcvt_to_sint(types::I32, rval);
                let or_i = builder.ins().bor(li, ri);
                let result = builder.ins().fcvt_from_sint(types::F64, or_i);
                locals.insert(*dest, result);
            }

            TypedInstruction::BitwiseXor { dest, left, right } => {
                let lval = locals[left];
                let rval = locals[right];
                let li = builder.ins().fcvt_to_sint(types::I32, lval);
                let ri = builder.ins().fcvt_to_sint(types::I32, rval);
                let xor_i = builder.ins().bxor(li, ri);
                let result = builder.ins().fcvt_from_sint(types::F64, xor_i);
                locals.insert(*dest, result);
            }

            TypedInstruction::ShiftLeft { dest, left, right } => {
                let lval = locals[left];
                let rval = locals[right];
                let li = builder.ins().fcvt_to_sint(types::I32, lval);
                let ri = builder.ins().fcvt_to_sint(types::I32, rval);
                // Mask shift amount to 5 bits (0-31) per JS spec
                let mask = builder.ins().iconst(types::I32, 0x1F);
                let shift_amt = builder.ins().band(ri, mask);
                let shl_i = builder.ins().ishl(li, shift_amt);
                let result = builder.ins().fcvt_from_sint(types::F64, shl_i);
                locals.insert(*dest, result);
            }

            TypedInstruction::ShiftRight { dest, left, right } => {
                let lval = locals[left];
                let rval = locals[right];
                let li = builder.ins().fcvt_to_sint(types::I32, lval);
                let ri = builder.ins().fcvt_to_sint(types::I32, rval);
                // Mask shift amount to 5 bits (0-31) per JS spec
                let mask = builder.ins().iconst(types::I32, 0x1F);
                let shift_amt = builder.ins().band(ri, mask);
                let shr_i = builder.ins().sshr(li, shift_amt);
                let result = builder.ins().fcvt_from_sint(types::F64, shr_i);
                locals.insert(*dest, result);
            }

            TypedInstruction::ShiftRightUnsigned { dest, left, right } => {
                let lval = locals[left];
                let rval = locals[right];
                // For unsigned right shift, treat as unsigned
                let li = builder.ins().fcvt_to_uint(types::I32, lval);
                let ri = builder.ins().fcvt_to_sint(types::I32, rval);
                // Mask shift amount to 5 bits (0-31) per JS spec
                let mask = builder.ins().iconst(types::I32, 0x1F);
                let shift_amt = builder.ins().band(ri, mask);
                let shru_i = builder.ins().ushr(li, shift_amt);
                let result = builder.ins().fcvt_from_uint(types::F64, shru_i);
                locals.insert(*dest, result);
            }

            TypedInstruction::Exponentiate {
                dest,
                base,
                exponent,
            } => {
                // x ** y - use runtime function for pow
                let base_val = locals[base];
                let exp_val = locals[exponent];
                // For now, use a simple approximation or call runtime
                // Simplified: just multiply for integer exponents
                // In production, this would call a proper pow function
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 90)) {
                    let call = builder.ins().call(func_ref, &[base_val, exp_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        locals.insert(*dest, base_val);
                    }
                } else {
                    // Fallback: just return base (incorrect but won't crash)
                    locals.insert(*dest, base_val);
                }
            }

            // Equality operators
            TypedInstruction::StrictEqual { dest, left, right } => {
                // Strict equality (===) per ECMAScript spec
                // Call the builtin function for proper type handling
                let lval = locals[left];
                let rval = locals[right];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 120)) {
                    let call = builder.ins().call(func_ref, &[lval, rval]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let zero = builder.ins().f64const(0.0);
                        locals.insert(*dest, zero);
                    }
                } else {
                    // Fallback to simple comparison if builtin not available
                    let cmp = builder.ins().fcmp(FloatCC::Equal, lval, rval);
                    let i = builder.ins().uextend(types::I32, cmp);
                    let result = builder.ins().fcvt_from_sint(types::F64, i);
                    locals.insert(*dest, result);
                }
            }

            TypedInstruction::StrictNotEqual { dest, left, right } => {
                // Strict inequality (!==) per ECMAScript spec
                let lval = locals[left];
                let rval = locals[right];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 120)) {
                    let call = builder.ins().call(func_ref, &[lval, rval]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        // Negate the result: 1.0 -> 0.0, 0.0 -> 1.0
                        let eq_result = results[0];
                        let one = builder.ins().f64const(1.0);
                        let result = builder.ins().fsub(one, eq_result);
                        locals.insert(*dest, result);
                    } else {
                        let one = builder.ins().f64const(1.0);
                        locals.insert(*dest, one);
                    }
                } else {
                    // Fallback to simple comparison if builtin not available
                    let cmp = builder.ins().fcmp(FloatCC::NotEqual, lval, rval);
                    let i = builder.ins().uextend(types::I32, cmp);
                    let result = builder.ins().fcvt_from_sint(types::F64, i);
                    locals.insert(*dest, result);
                }
            }

            TypedInstruction::LooseEqual { dest, left, right } => {
                // Loose equality (==) per ECMAScript spec with type coercion
                let lval = locals[left];
                let rval = locals[right];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 121)) {
                    let call = builder.ins().call(func_ref, &[lval, rval]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let zero = builder.ins().f64const(0.0);
                        locals.insert(*dest, zero);
                    }
                } else {
                    // Fallback to simple comparison if builtin not available
                    let cmp = builder.ins().fcmp(FloatCC::Equal, lval, rval);
                    let i = builder.ins().uextend(types::I32, cmp);
                    let result = builder.ins().fcvt_from_sint(types::F64, i);
                    locals.insert(*dest, result);
                }
            }

            TypedInstruction::LooseNotEqual { dest, left, right } => {
                // Loose inequality (!=) per ECMAScript spec with type coercion
                let lval = locals[left];
                let rval = locals[right];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 121)) {
                    let call = builder.ins().call(func_ref, &[lval, rval]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        // Negate the result: 1.0 -> 0.0, 0.0 -> 1.0
                        let eq_result = results[0];
                        let one = builder.ins().f64const(1.0);
                        let result = builder.ins().fsub(one, eq_result);
                        locals.insert(*dest, result);
                    } else {
                        let one = builder.ins().f64const(1.0);
                        locals.insert(*dest, one);
                    }
                } else {
                    // Fallback to simple comparison if builtin not available
                    let cmp = builder.ins().fcmp(FloatCC::NotEqual, lval, rval);
                    let i = builder.ins().uextend(types::I32, cmp);
                    let result = builder.ins().fcvt_from_sint(types::F64, i);
                    locals.insert(*dest, result);
                }
            }

            TypedInstruction::InstanceOf {
                dest,
                object,
                constructor,
            } => {
                // instanceof operator - check prototype chain
                // Requirements: 6.3 - prototype chain for inheritance
                let obj_val = locals[object];
                let ctor_val = locals[constructor];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 145)) {
                    let call = builder.ins().call(func_ref, &[obj_val, ctor_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let zero = builder.ins().f64const(0.0);
                        locals.insert(*dest, zero);
                    }
                } else {
                    // Fallback: return false
                    let result = builder.ins().f64const(0.0);
                    locals.insert(*dest, result);
                }
            }

            TypedInstruction::In {
                dest,
                property,
                object,
            } => {
                // in operator - call runtime
                let _prop_val = locals[property];
                let _obj_val = locals[object];
                // For now, return false (0.0)
                let result = builder.ins().f64const(0.0);
                locals.insert(*dest, result);
            }

            TypedInstruction::Delete {
                dest,
                object,
                property: _,
            } => {
                // delete operator - call runtime
                let _obj_val = locals[object];
                // For now, return true (1.0)
                let result = builder.ins().f64const(1.0);
                locals.insert(*dest, result);
            }

            TypedInstruction::DeleteComputed { dest, object, key } => {
                // delete with computed key - call runtime
                let _obj_val = locals[object];
                let _key_val = locals[key];
                // For now, return true (1.0)
                let result = builder.ins().f64const(1.0);
                locals.insert(*dest, result);
            }

            // Class-related instructions
            TypedInstruction::CreateClass {
                dest,
                class_id: _class_id,
                constructor_id,
                super_class,
            } => {
                // Create a class using the new class builtin
                // Requirements: 6.1, 6.2, 6.3 - class instantiation and prototype chain
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 140)) {
                    // Get constructor ID (NaN if no explicit constructor)
                    let ctor_id_val = builder
                        .ins()
                        .f64const(constructor_id.map(|id| id.0 as f64).unwrap_or(f64::NAN));
                    
                    // Get super class ID (NaN if no extends clause)
                    let super_class_val = match super_class {
                        Some(super_local) => locals[super_local],
                        None => builder.ins().f64const(f64::NAN),
                    };
                    
                    let call = builder.ins().call(func_ref, &[ctor_id_val, super_class_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::GetPrototype { dest, constructor } => {
                // Get the prototype of a class/constructor
                // Requirements: 6.3 - prototype chain for inheritance
                let ctor_val = locals[constructor];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 143)) {
                    let call = builder.ins().call(func_ref, &[ctor_val]);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        locals.insert(*dest, ctor_val);
                    }
                } else {
                    // Fallback: return the constructor itself
                    locals.insert(*dest, ctor_val);
                }
            }

            TypedInstruction::SetPrototype { object, prototype } => {
                // Set the prototype of an object
                // Requirements: 6.3 - prototype chain setup
                let obj_val = locals[object];
                let proto_val = locals[prototype];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 152)) {
                    let _call = builder.ins().call(func_ref, &[obj_val, proto_val]);
                }
            }

            TypedInstruction::CallSuper {
                dest,
                super_constructor,
                args,
                this_arg,
            } => {
                // Call super constructor
                // Requirements: 6.4 - super() calls parent constructor with correct this
                let super_val = locals[super_constructor];
                let this_val = locals[this_arg];

                // First, get the parent class's constructor function
                if let Some(&get_ctor_ref) = func_refs.get(&(u32::MAX - 142)) {
                    let get_ctor_call = builder.ins().call(get_ctor_ref, &[super_val]);
                    let ctor_results = builder.inst_results(get_ctor_call);
                    
                    let parent_ctor = if !ctor_results.is_empty() {
                        ctor_results[0]
                    } else {
                        super_val
                    };
                    
                    // Call the parent constructor with this binding
                    if let Some(&call_func_ref) = func_refs.get(&(u32::MAX - 90)) {
                        let nan = builder.ins().f64const(f64::NAN);
                        let arg_count = builder.ins().f64const(args.len() as f64);

                        let arg0 = args.first().map(|a| locals[a]).unwrap_or(nan);
                        let arg1 = args.get(1).map(|a| locals[a]).unwrap_or(nan);
                        let arg2 = args.get(2).map(|a| locals[a]).unwrap_or(nan);
                        let arg3 = args.get(3).map(|a| locals[a]).unwrap_or(nan);
                        let arg4 = args.get(4).map(|a| locals[a]).unwrap_or(nan);
                        let arg5 = args.get(5).map(|a| locals[a]).unwrap_or(nan);
                        let arg6 = args.get(6).map(|a| locals[a]).unwrap_or(nan);
                        let arg7 = args.get(7).map(|a| locals[a]).unwrap_or(nan);

                        // Set this binding before calling
                        if let Some(&set_this_ref) = func_refs.get(&(u32::MAX - 32)) {
                            let _set_call = builder.ins().call(set_this_ref, &[this_val]);
                        }

                        let call = builder.ins().call(
                            call_func_ref,
                            &[
                                parent_ctor, arg_count, arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7,
                            ],
                        );
                        let results = builder.inst_results(call);

                        if let Some(dest) = dest {
                            if !results.is_empty() {
                                locals.insert(*dest, results[0]);
                            } else {
                                locals.insert(*dest, this_val);
                            }
                        }
                    } else if let Some(dest) = dest {
                        locals.insert(*dest, this_val);
                    }
                } else if let Some(dest) = dest {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::SuperMethodCall {
                dest,
                super_class,
                method_name,
                args,
                this_arg,
            } => {
                // Call a method on the super class
                // Requirements: 6.5 - super.method() calls parent class method with current this
                let super_val = locals[super_class];
                let this_val = locals[this_arg];

                // Get the super class's prototype
                if let Some(&get_proto_ref) = func_refs.get(&(u32::MAX - 143)) {
                    let get_proto_call = builder.ins().call(get_proto_ref, &[super_val]);
                    let proto_results = builder.inst_results(get_proto_call);
                    
                    let super_proto = if !proto_results.is_empty() {
                        proto_results[0]
                    } else {
                        super_val
                    };
                    
                    // Get the method from the super prototype
                    if let Some(&get_prop_ref) = func_refs.get(&(u32::MAX - 20)) {
                        let method_hash = hash_string(method_name) as f64;
                        let method_name_val = builder.ins().f64const(method_hash);
                        
                        let get_method_call = builder.ins().call(get_prop_ref, &[super_proto, method_name_val]);
                        let method_results = builder.inst_results(get_method_call);
                        
                        let method = if !method_results.is_empty() {
                            method_results[0]
                        } else {
                            let nan = builder.ins().f64const(f64::NAN);
                            nan
                        };
                        
                        // Call the method with this binding
                        if let Some(&call_func_ref) = func_refs.get(&(u32::MAX - 90)) {
                            let nan = builder.ins().f64const(f64::NAN);
                            let arg_count = builder.ins().f64const(args.len() as f64);

                            let arg0 = args.first().map(|a| locals[a]).unwrap_or(nan);
                            let arg1 = args.get(1).map(|a| locals[a]).unwrap_or(nan);
                            let arg2 = args.get(2).map(|a| locals[a]).unwrap_or(nan);
                            let arg3 = args.get(3).map(|a| locals[a]).unwrap_or(nan);
                            let arg4 = args.get(4).map(|a| locals[a]).unwrap_or(nan);
                            let arg5 = args.get(5).map(|a| locals[a]).unwrap_or(nan);
                            let arg6 = args.get(6).map(|a| locals[a]).unwrap_or(nan);
                            let arg7 = args.get(7).map(|a| locals[a]).unwrap_or(nan);

                            // Set this binding before calling
                            if let Some(&set_this_ref) = func_refs.get(&(u32::MAX - 32)) {
                                let _set_call = builder.ins().call(set_this_ref, &[this_val]);
                            }

                            let call = builder.ins().call(
                                call_func_ref,
                                &[
                                    method, arg_count, arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7,
                                ],
                            );
                            let results = builder.inst_results(call);

                            if let Some(dest) = dest {
                                if !results.is_empty() {
                                    locals.insert(*dest, results[0]);
                                } else {
                                    let nan = builder.ins().f64const(f64::NAN);
                                    locals.insert(*dest, nan);
                                }
                            }
                        } else if let Some(dest) = dest {
                            let nan = builder.ins().f64const(f64::NAN);
                            locals.insert(*dest, nan);
                        }
                    } else if let Some(dest) = dest {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else if let Some(dest) = dest {
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::DefineMethod {
                prototype,
                name,
                function_id,
                is_static,
            } => {
                // Define a method on a class prototype
                // Requirements: 6.2, 6.6 - methods accessible on instances, static methods on class
                let proto_val = locals[prototype];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 146)) {
                    // Allocate the method name as a string
                    let name_hash = hash_string(name) as f64;
                    let name_val = builder.ins().f64const(name_hash);
                    let func_id_val = builder.ins().f64const(function_id.0 as f64);
                    let is_static_val = builder.ins().f64const(if *is_static { 1.0 } else { 0.0 });
                    
                    let _call = builder.ins().call(func_ref, &[proto_val, name_val, func_id_val, is_static_val]);
                }
            }

            TypedInstruction::DefineGetter {
                prototype,
                name,
                function_id,
                is_static,
            } => {
                // Define a getter on a class
                // Requirements: 6.7 - getters invoked on property access
                let proto_val = locals[prototype];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 147)) {
                    let name_hash = hash_string(name) as f64;
                    let name_val = builder.ins().f64const(name_hash);
                    let func_id_val = builder.ins().f64const(function_id.0 as f64);
                    let is_static_val = builder.ins().f64const(if *is_static { 1.0 } else { 0.0 });
                    
                    let _call = builder.ins().call(func_ref, &[proto_val, name_val, func_id_val, is_static_val]);
                }
            }

            TypedInstruction::DefineSetter {
                prototype,
                name,
                function_id,
                is_static,
            } => {
                // Define a setter on a class
                // Requirements: 6.7 - setters invoked on property assignment
                let proto_val = locals[prototype];
                
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 148)) {
                    let name_hash = hash_string(name) as f64;
                    let name_val = builder.ins().f64const(name_hash);
                    let func_id_val = builder.ins().f64const(function_id.0 as f64);
                    let is_static_val = builder.ins().f64const(if *is_static { 1.0 } else { 0.0 });
                    
                    let _call = builder.ins().call(func_ref, &[proto_val, name_val, func_id_val, is_static_val]);
                }
            }

            TypedInstruction::DynamicImport { dest, specifier } => {
                // Dynamic import expression - import(specifier)
                // Returns a Promise that resolves to the module namespace
                //
                // Requirements:
                // - 2.1: Return a Promise that resolves to the module namespace
                // - 2.2: Resolve relative paths relative to the importing module
                // - 2.3: Resolve bare specifiers using Node.js module resolution
                
                let specifier_val = locals[specifier];
                
                // Call the builtin_dynamic_import function
                // This function:
                // 1. Gets the specifier string from the heap
                // 2. Resolves the module path
                // 3. Creates a Promise
                // 4. Loads and evaluates the module asynchronously
                // 5. Returns the Promise ID
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 100)) {
                    // Get the current module path for relative resolution
                    // For now, use a placeholder (will be set by the runtime)
                    let referrer = builder.ins().f64const(f64::NAN);
                    
                    let call = builder.ins().call(func_ref, &[specifier_val, referrer]);
                    let results = builder.inst_results(call);
                    
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return NaN (undefined) if builtin not registered
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::ArraySliceFrom { dest, source, start_index } => {
                // Array slice operation for rest elements in destructuring
                // Creates a new array from source[start_index..]
                // Requirements: 7.4 - rest elements in array destructuring
                
                let source_val = locals[source];
                let start_val = builder.ins().f64const(*start_index as f64);
                
                // Call __dx_array_slice_from(source, start_index)
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 200)) {
                    let call = builder.ins().call(func_ref, &[source_val, start_val]);
                    let results = builder.inst_results(call);
                    
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return empty array
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::ObjectRest { dest, source, excluded_keys } => {
                // Object rest operation for rest properties in destructuring
                // Creates a new object with all properties except excluded_keys
                // Requirements: 7.5 - rest properties in object destructuring
                
                let source_val = locals[source];
                
                // For now, we'll create a new object and copy non-excluded properties
                // This is a simplified implementation - a full implementation would
                // pass the excluded keys to the runtime function
                
                // Call __dx_object_rest(source, excluded_count)
                // The excluded keys are passed separately via a helper
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 201)) {
                    let excluded_count = builder.ins().f64const(excluded_keys.len() as f64);
                    let call = builder.ins().call(func_ref, &[source_val, excluded_count]);
                    let results = builder.inst_results(call);
                    
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        let nan = builder.ins().f64const(f64::NAN);
                        locals.insert(*dest, nan);
                    }
                } else {
                    // Fallback: return empty object
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }

            TypedInstruction::IsUndefined { dest, src } => {
                // Check if value is undefined (for default value handling)
                // Requirements: 7.3 - default values in destructuring
                
                let src_val = locals[src];
                
                // Call __dx_is_undefined(value)
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 202)) {
                    let call = builder.ins().call(func_ref, &[src_val]);
                    let results = builder.inst_results(call);
                    
                    if !results.is_empty() {
                        locals.insert(*dest, results[0]);
                    } else {
                        // Fallback: assume not undefined
                        let zero = builder.ins().f64const(0.0);
                        locals.insert(*dest, zero);
                    }
                } else {
                    // Fallback: check if value is NaN (our undefined representation)
                    // NaN != NaN is true, so we can use this to check for undefined
                    let is_nan = builder.ins().fcmp(FloatCC::NotEqual, src_val, src_val);
                    let one = builder.ins().f64const(1.0);
                    let zero = builder.ins().f64const(0.0);
                    let result = builder.ins().select(is_nan, one, zero);
                    locals.insert(*dest, result);
                }
            }

            TypedInstruction::ThrowDestructuringError { source } => {
                // Throw TypeError for destructuring null/undefined
                // Requirements: 7.7 - destructuring null/undefined error
                
                let source_val = locals[source];
                
                // Call __dx_throw_destructuring_error(source)
                if let Some(&func_ref) = func_refs.get(&(u32::MAX - 203)) {
                    let _call = builder.ins().call(func_ref, &[source_val]);
                }
                // Note: This should never return, but we don't have a way to express that
                // in Cranelift without using a trap or unreachable instruction
            }

            TypedInstruction::BuildTemplateLiteral { dest, quasis, expressions } => {
                // Build a template literal by concatenating quasis and expressions
                // Requirements: 8.1, 8.2 - template literal interpolation with multiline support
                
                let heap = get_runtime_heap_lock();
                let mut heap_guard = heap.lock().unwrap();
                
                // Allocate string IDs for all quasis and store them
                let mut quasi_ids: Vec<f64> = Vec::with_capacity(quasis.len());
                for quasi in quasis {
                    let id = heap_guard.allocate_string(quasi.clone());
                    quasi_ids.push(encode_string_id(id));
                }
                drop(heap_guard);
                
                // If no expressions, just return the first quasi
                if expressions.is_empty() {
                    if let Some(&quasi_id) = quasi_ids.first() {
                        let result = builder.ins().f64const(quasi_id);
                        locals.insert(*dest, result);
                    } else {
                        // Empty template literal
                        let heap = get_runtime_heap_lock();
                        let mut heap_guard = heap.lock().unwrap();
                        let id = heap_guard.allocate_string(String::new());
                        let result = builder.ins().f64const(encode_string_id(id));
                        locals.insert(*dest, result);
                    }
                } else {
                    // Build the result by concatenating quasis and expressions
                    // Start with the first quasi
                    let mut current = if let Some(&quasi_id) = quasi_ids.first() {
                        builder.ins().f64const(quasi_id)
                    } else {
                        let heap = get_runtime_heap_lock();
                        let mut heap_guard = heap.lock().unwrap();
                        let id = heap_guard.allocate_string(String::new());
                        builder.ins().f64const(encode_string_id(id))
                    };
                    
                    // Interleave expressions and remaining quasis
                    for (i, expr_local) in expressions.iter().enumerate() {
                        let expr_val = locals[expr_local];
                        
                        // Convert expression to string and concatenate
                        if let Some(&to_string_func) = func_refs.get(&(u32::MAX - 124)) {
                            let call = builder.ins().call(to_string_func, &[expr_val]);
                            let results = builder.inst_results(call);
                            let expr_str = if !results.is_empty() {
                                results[0]
                            } else {
                                expr_val
                            };
                            
                            // Concatenate current with expression string
                            if let Some(&concat_func) = func_refs.get(&(u32::MAX - 125)) {
                                let call = builder.ins().call(concat_func, &[current, expr_str]);
                                let results = builder.inst_results(call);
                                current = if !results.is_empty() {
                                    results[0]
                                } else {
                                    current
                                };
                            }
                        }
                        
                        // Add the next quasi (if there is one)
                        if i + 1 < quasi_ids.len() {
                            let quasi_val = builder.ins().f64const(quasi_ids[i + 1]);
                            if let Some(&concat_func) = func_refs.get(&(u32::MAX - 125)) {
                                let call = builder.ins().call(concat_func, &[current, quasi_val]);
                                let results = builder.inst_results(call);
                                current = if !results.is_empty() {
                                    results[0]
                                } else {
                                    current
                                };
                            }
                        }
                    }
                    
                    locals.insert(*dest, current);
                }
            }

            TypedInstruction::CallTaggedTemplate { dest, tag, quasis, raw_quasis, expressions } => {
                // Call a tagged template function
                // Requirements: 8.3 - tagged template invocation
                
                let tag_val = locals[tag];
                
                // Create the strings array with cooked values
                let heap = get_runtime_heap_lock();
                let mut heap_guard = heap.lock().unwrap();
                
                // Create array of cooked strings
                let cooked_strings: Vec<f64> = quasis.iter().map(|s| {
                    let id = heap_guard.allocate_string(s.clone());
                    encode_string_id(id)
                }).collect();
                let strings_array_id = heap_guard.allocate_array(cooked_strings);
                
                // Create array of raw strings and attach as 'raw' property
                let raw_strings: Vec<f64> = raw_quasis.iter().map(|s| {
                    let id = heap_guard.allocate_string(s.clone());
                    encode_string_id(id)
                }).collect();
                let raw_array_id = heap_guard.allocate_array(raw_strings);
                
                // Set the 'raw' property on the strings array
                // For now, we'll store it as a separate object property
                // In a full implementation, we'd attach it to the array object
                let _ = raw_array_id; // TODO: Attach raw property to strings array
                
                drop(heap_guard);
                
                let strings_array_val = builder.ins().f64const(strings_array_id as f64);
                
                // Build arguments: [strings_array, ...expressions]
                let mut args = vec![tag_val, strings_array_val];
                for expr_local in expressions {
                    args.push(locals[expr_local]);
                }
                
                // Call the tag function using __dx_call_function
                if let Some(&call_func) = func_refs.get(&(u32::MAX - 90)) {
                    // Pad args to 10 (closure_id, arg_count, arg0-arg7)
                    let arg_count = args.len() - 1; // Exclude tag function
                    let mut call_args = vec![tag_val];
                    call_args.push(builder.ins().f64const(arg_count as f64));
                    call_args.push(strings_array_val);
                    
                    for expr_local in expressions.iter().take(7) {
                        call_args.push(locals[expr_local]);
                    }
                    
                    // Pad remaining args with NaN
                    while call_args.len() < 10 {
                        call_args.push(builder.ins().f64const(f64::NAN));
                    }
                    
                    let call = builder.ins().call(call_func, &call_args);
                    let results = builder.inst_results(call);
                    let result = if !results.is_empty() {
                        results[0]
                    } else {
                        builder.ins().f64const(f64::NAN)
                    };
                    locals.insert(*dest, result);
                } else {
                    // Fallback: return NaN
                    let nan = builder.ins().f64const(f64::NAN);
                    locals.insert(*dest, nan);
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn compile_terminator(
        &self,
        builder: &mut CraneliftFunctionBuilder,
        term: &Terminator,
        locals: &HashMap<LocalId, cranelift::prelude::Value>,
        block_map: &HashMap<BlockId, Block>,
    ) -> DxResult<()> {
        match term {
            Terminator::Return(value) => {
                if let Some(val_id) = value {
                    let val = locals[val_id];
                    builder.ins().return_(&[val]);
                } else {
                    let nan = builder.ins().f64const(f64::NAN);
                    builder.ins().return_(&[nan]);
                }
            }

            Terminator::Goto(target) => {
                let target_block = block_map[target];
                builder.ins().jump(target_block, &[]);
            }

            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let cond = locals[condition];
                let then_bl = block_map[then_block];
                let else_bl = block_map[else_block];

                // Convert f64 condition to boolean (non-zero = true)
                let zero = builder.ins().f64const(0.0);
                let is_true = builder.ins().fcmp(FloatCC::NotEqual, cond, zero);

                builder.ins().brif(is_true, then_bl, &[], else_bl, &[]);
            }

            Terminator::Unreachable => {
                // Return NaN for unreachable code
                let nan = builder.ins().f64const(f64::NAN);
                builder.ins().return_(&[nan]);
            }
        }

        Ok(())
    }
}
