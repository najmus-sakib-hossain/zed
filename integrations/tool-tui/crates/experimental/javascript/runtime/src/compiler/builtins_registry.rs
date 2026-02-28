//! Built-in JavaScript Objects and Functions
//!
//! This module provides native implementations of core JavaScript built-ins:
//! - Object (keys, values, entries, assign, freeze, etc.)
//! - Array (map, filter, reduce, sort, etc.)
//! - String (split, join, slice, replace, match, etc.)
//! - Number (toFixed, toString, parseInt, parseFloat)
//! - Math (floor, ceil, sqrt, sin, cos, random, etc.)
//! - JSON (parse, stringify)
//! - console (log, warn, error, time, etc.)
//! - Date, RegExp, Map, Set, etc.
//!
//! # Error Handling
//!
//! Built-in functions use the structured exception system for proper error reporting.
//! When an error occurs, functions call `throw_type_error`, `throw_range_error`, etc.
//! to set a structured exception with stack trace information, then return a sentinel
//! value (typically `Value::Undefined` or `f64::NAN`).

use crate::compiler::codegen::{throw_syntax_error, throw_type_error};
use crate::value::Value;
use std::collections::{HashMap, HashSet};

/// Built-in function registry
pub struct BuiltinRegistry {
    functions: HashMap<String, BuiltinFunction>,
}

/// A built-in function pointer
pub type BuiltinFunction = fn(&[Value]) -> Value;

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        registry.register_all();
        registry
    }

    /// Register all built-in functions
    fn register_all(&mut self) {
        // Console methods
        self.register("console.log", builtin_console_log);
        self.register("console.warn", builtin_console_warn);
        self.register("console.error", builtin_console_error);
        self.register("console.time", builtin_console_time);
        self.register("console.timeEnd", builtin_console_time_end);
        self.register("console.timeLog", builtin_console_time_log);

        // Math methods
        self.register("Math.floor", builtin_math_floor);
        self.register("Math.ceil", builtin_math_ceil);
        self.register("Math.round", builtin_math_round);
        self.register("Math.sqrt", builtin_math_sqrt);
        self.register("Math.abs", builtin_math_abs);
        self.register("Math.sin", builtin_math_sin);
        self.register("Math.cos", builtin_math_cos);
        self.register("Math.tan", builtin_math_tan);
        self.register("Math.min", builtin_math_min);
        self.register("Math.max", builtin_math_max);
        self.register("Math.pow", builtin_math_pow);
        self.register("Math.random", builtin_math_random);

        // Object methods
        self.register("Object.keys", builtin_object_keys);
        self.register("Object.values", builtin_object_values);
        self.register("Object.entries", builtin_object_entries);
        self.register("Object.assign", builtin_object_assign);
        self.register("Object.freeze", builtin_object_freeze);
        self.register("Object.seal", builtin_object_seal);

        // Array methods
        self.register("Array.isArray", builtin_array_is_array);
        self.register("Array.from", builtin_array_from);
        self.register("Array.of", builtin_array_of);

        // String methods
        self.register("String.fromCharCode", builtin_string_from_char_code);

        // Number methods
        self.register("Number.isNaN", builtin_number_is_nan);
        self.register("Number.isFinite", builtin_number_is_finite);
        self.register("Number.parseInt", builtin_number_parse_int);
        self.register("Number.parseFloat", builtin_number_parse_float);

        // JSON methods
        self.register("JSON.parse", builtin_json_parse);
        self.register("JSON.stringify", builtin_json_stringify);

        // Global functions
        self.register("parseInt", builtin_parse_int);
        self.register("parseFloat", builtin_parse_float);
        self.register("isNaN", builtin_is_nan);
        self.register("isFinite", builtin_is_finite);

        // Promise methods
        self.register("Promise.resolve", builtin_promise_resolve);
        self.register("Promise.reject", builtin_promise_reject);
        self.register("Promise.all", builtin_promise_all);
        self.register("Promise.race", builtin_promise_race);
        self.register("Promise.any", builtin_promise_any);
        self.register("Promise.allSettled", builtin_promise_all_settled);

        // Process methods
        self.register("process.memoryUsage", builtin_process_memory_usage);

        // DX global object methods
        self.register("dx.features", builtin_dx_features);
        self.register("dx.version", builtin_dx_version);
    }

    fn register(&mut self, name: &str, func: BuiltinFunction) {
        self.functions.insert(name.to_string(), func);
    }

    pub fn get(&self, name: &str) -> Option<BuiltinFunction> {
        self.functions.get(name).copied()
    }
}

// ============================================================================
// Console Methods
// ============================================================================

fn builtin_console_log(args: &[Value]) -> Value {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{arg}");
    }
    println!();
    Value::Undefined
}

fn builtin_console_warn(args: &[Value]) -> Value {
    eprint!("Warning: ");
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            eprint!(" ");
        }
        eprint!("{arg}");
    }
    eprintln!();
    Value::Undefined
}

fn builtin_console_error(args: &[Value]) -> Value {
    eprint!("Error: ");
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            eprint!(" ");
        }
        eprint!("{arg}");
    }
    eprintln!();
    Value::Undefined
}

fn builtin_console_time(args: &[Value]) -> Value {
    let label = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => "default".to_string(),
    };
    crate::runtime::console::console_time(&label);
    Value::Undefined
}

fn builtin_console_time_end(args: &[Value]) -> Value {
    let label = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => "default".to_string(),
    };
    crate::runtime::console::console_time_end(&label);
    Value::Undefined
}

fn builtin_console_time_log(args: &[Value]) -> Value {
    let label = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => "default".to_string(),
    };
    let data = if args.len() > 1 { &args[1..] } else { &[] };
    crate::runtime::console::console_time_log(&label, data);
    Value::Undefined
}

// ============================================================================
// Math Methods
// ============================================================================

fn builtin_math_floor(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.floor())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_ceil(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.ceil())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_round(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.round())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_sqrt(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.sqrt())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_abs(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.abs())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_sin(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.sin())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_cos(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.cos())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_tan(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Number(n.tan())
    } else {
        Value::Number(f64::NAN)
    }
}

fn builtin_math_min(args: &[Value]) -> Value {
    let mut min = f64::INFINITY;
    for arg in args {
        if let Value::Number(n) = arg {
            if n < &min {
                min = *n;
            }
        }
    }
    Value::Number(min)
}

fn builtin_math_max(args: &[Value]) -> Value {
    let mut max = f64::NEG_INFINITY;
    for arg in args {
        if let Value::Number(n) = arg {
            if n > &max {
                max = *n;
            }
        }
    }
    Value::Number(max)
}

fn builtin_math_pow(args: &[Value]) -> Value {
    if args.len() >= 2 {
        if let (Value::Number(base), Value::Number(exp)) = (&args[0], &args[1]) {
            return Value::Number(base.powf(*exp));
        }
    }
    Value::Number(f64::NAN)
}

fn builtin_math_random(_args: &[Value]) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let random = ((nanos % 1_000_000) as f64) / 1_000_000.0;
    Value::Number(random)
}

// ============================================================================
// Object Methods
// ============================================================================

fn builtin_object_keys(args: &[Value]) -> Value {
    match args.first() {
        Some(Value::Object(obj)) => {
            let keys: Vec<Value> = obj.keys_owned().into_iter().map(Value::String).collect();
            Value::Array(keys)
        }
        Some(other) => {
            // TypeError: Object.keys called on non-object
            throw_type_error(format!(
                "Object.keys called on non-object (received {})",
                other.type_name()
            ));
            Value::Array(vec![])
        }
        None => {
            throw_type_error("Object.keys requires an argument");
            Value::Array(vec![])
        }
    }
}

fn builtin_object_values(args: &[Value]) -> Value {
    match args.first() {
        Some(Value::Object(obj)) => {
            let values = obj.values_cloned();
            Value::Array(values)
        }
        Some(other) => {
            // TypeError: Object.values called on non-object
            throw_type_error(format!(
                "Object.values called on non-object (received {})",
                other.type_name()
            ));
            Value::Array(vec![])
        }
        None => {
            throw_type_error("Object.values requires an argument");
            Value::Array(vec![])
        }
    }
}

fn builtin_object_entries(args: &[Value]) -> Value {
    match args.first() {
        Some(Value::Object(obj)) => {
            let entries: Vec<Value> = obj
                .entries_cloned()
                .into_iter()
                .map(|(k, v)| Value::Array(vec![Value::String(k), v]))
                .collect();
            Value::Array(entries)
        }
        Some(other) => {
            // TypeError: Object.entries called on non-object
            throw_type_error(format!(
                "Object.entries called on non-object (received {})",
                other.type_name()
            ));
            Value::Array(vec![])
        }
        None => {
            throw_type_error("Object.entries requires an argument");
            Value::Array(vec![])
        }
    }
}

fn builtin_object_assign(args: &[Value]) -> Value {
    if args.is_empty() {
        throw_type_error("Object.assign requires at least one argument");
        return Value::Undefined;
    }

    // Get target object
    let mut target = match &args[0] {
        Value::Object(obj) => obj.clone(),
        other => {
            throw_type_error(format!(
                "Object.assign target must be an object (received {})",
                other.type_name()
            ));
            return Value::Undefined;
        }
    };

    // Copy properties from all source objects
    for source in args.iter().skip(1) {
        if let Value::Object(src_obj) = source {
            target.assign_from(src_obj);
        }
        // Non-object sources are silently ignored per spec
    }

    Value::Object(target)
}

fn builtin_object_freeze(args: &[Value]) -> Value {
    match args.first() {
        Some(Value::Object(obj)) => {
            let mut frozen_obj = obj.clone();
            frozen_obj.freeze();
            Value::Object(frozen_obj)
        }
        Some(other) => other.clone(), // Non-objects are returned as-is
        None => Value::Undefined,
    }
}

fn builtin_object_seal(args: &[Value]) -> Value {
    match args.first() {
        Some(Value::Object(obj)) => {
            let mut sealed_obj = obj.clone();
            sealed_obj.seal();
            Value::Object(sealed_obj)
        }
        Some(other) => other.clone(), // Non-objects are returned as-is
        None => Value::Undefined,
    }
}

// ============================================================================
// Array Methods
// ============================================================================

fn builtin_array_is_array(args: &[Value]) -> Value {
    match args.first() {
        Some(Value::Array(_)) => Value::Boolean(true),
        _ => Value::Boolean(false),
    }
}

fn builtin_array_from(args: &[Value]) -> Value {
    match args.first() {
        // Array from array - return a copy
        Some(Value::Array(arr)) => Value::Array(arr.clone()),
        // Array from string - split into characters
        Some(Value::String(s)) => {
            let chars: Vec<Value> = s.chars().map(|c| Value::String(c.to_string())).collect();
            Value::Array(chars)
        }
        // Array from object with length property (array-like)
        Some(Value::Object(obj)) => {
            if let Some(Value::Number(len)) = obj.get("length") {
                let len = *len as usize;
                let mut result = Vec::with_capacity(len);
                for i in 0..len {
                    let key = i.to_string();
                    let value = obj.get(&key).cloned().unwrap_or(Value::Undefined);
                    result.push(value);
                }
                Value::Array(result)
            } else {
                Value::Array(vec![])
            }
        }
        // Non-iterable types - throw TypeError
        Some(other) => {
            throw_type_error(format!(
                "Array.from requires an array-like or iterable object (received {})",
                other.type_name()
            ));
            Value::Array(vec![])
        }
        None => {
            throw_type_error("Array.from requires an argument");
            Value::Array(vec![])
        }
    }
}

fn builtin_array_of(args: &[Value]) -> Value {
    // Array.of creates an array from all arguments
    Value::Array(args.to_vec())
}

// ============================================================================
// String Methods
// ============================================================================

fn builtin_string_from_char_code(args: &[Value]) -> Value {
    let mut result = String::new();
    for arg in args {
        if let Value::Number(n) = arg {
            if let Some(ch) = char::from_u32(*n as u32) {
                result.push(ch);
            }
        }
    }
    Value::String(result)
}

// ============================================================================
// Number Methods
// ============================================================================

fn builtin_number_is_nan(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Boolean(n.is_nan())
    } else {
        Value::Boolean(false)
    }
}

fn builtin_number_is_finite(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Boolean(n.is_finite())
    } else {
        Value::Boolean(false)
    }
}

fn builtin_number_parse_int(args: &[Value]) -> Value {
    builtin_parse_int(args)
}

fn builtin_number_parse_float(args: &[Value]) -> Value {
    builtin_parse_float(args)
}

// ============================================================================
// JSON Methods
// ============================================================================

fn builtin_json_parse(args: &[Value]) -> Value {
    let json_str = match args.first() {
        Some(Value::String(s)) => s,
        Some(other) => {
            throw_syntax_error(format!(
                "JSON.parse requires a string argument (received {})",
                other.type_name()
            ));
            return Value::Undefined;
        }
        None => {
            throw_syntax_error("JSON.parse requires a string argument");
            return Value::Undefined;
        }
    };

    // Parse using serde_json
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(json_value) => json_to_value(json_value),
        Err(e) => {
            // Extract line and column from serde_json error if available
            let line = e.line();
            let column = e.column();
            throw_syntax_error(format!("JSON.parse: {} at line {}, column {}", e, line, column));
            Value::Undefined
        }
    }
}

/// Convert serde_json::Value to our Value type
fn json_to_value(json: serde_json::Value) -> Value {
    use crate::value::object::Object;

    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(f64::NAN)),
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => {
            let values: Vec<Value> = arr.into_iter().map(json_to_value).collect();
            Value::Array(values)
        }
        serde_json::Value::Object(map) => {
            let mut obj = Object::new();
            for (key, val) in map {
                obj.set(key, json_to_value(val));
            }
            Value::Object(obj)
        }
    }
}

fn builtin_json_stringify(args: &[Value]) -> Value {
    use std::collections::HashSet;

    let val = match args.first() {
        Some(v) => v,
        None => return Value::Undefined,
    };

    // Track visited objects for circular reference detection
    let mut visited = HashSet::new();

    match value_to_json_string(val, &mut visited) {
        Some(s) => Value::String(s),
        None => {
            // TypeError for circular references or non-serializable values
            throw_type_error("Converting circular structure to JSON");
            Value::Undefined
        }
    }
}

/// Convert Value to JSON string with circular reference detection
fn value_to_json_string(val: &Value, visited: &mut HashSet<usize>) -> Option<String> {
    match val {
        Value::Undefined => None, // undefined is not valid JSON
        Value::Null => Some("null".to_string()),
        Value::Boolean(b) => Some(if *b { "true" } else { "false" }.to_string()),
        Value::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                Some("null".to_string()) // JSON spec: NaN/Infinity become null
            } else {
                Some(n.to_string())
            }
        }
        Value::String(s) => Some(format!("\"{}\"", escape_json_string(s))),
        Value::Array(arr) => {
            let ptr = arr.as_ptr() as usize;
            if visited.contains(&ptr) {
                return None; // Circular reference detected
            }
            visited.insert(ptr);

            let mut parts = Vec::new();
            for item in arr {
                let json_item = value_to_json_string(item, visited).unwrap_or("null".to_string());
                parts.push(json_item);
            }

            visited.remove(&ptr);
            Some(format!("[{}]", parts.join(",")))
        }
        Value::Object(obj) => {
            let ptr = obj as *const _ as usize;
            if visited.contains(&ptr) {
                return None; // Circular reference detected
            }
            visited.insert(ptr);

            let mut parts = Vec::new();
            for (key, value) in obj.entries() {
                if let Some(json_value) = value_to_json_string(value, visited) {
                    parts.push(format!("\"{}\":{}", escape_json_string(key), json_value));
                }
            }

            visited.remove(&ptr);
            Some(format!("{{{}}}", parts.join(",")))
        }
        Value::Function(_) => None, // Functions are not serializable
        Value::Promise(_) => None,  // Promises are not serializable
        Value::Integer(i) => Some(i.to_string()),
        Value::Symbol(_) => None, // Symbols are not serializable
        Value::BigInt(b) => Some(format!("\"{}\"", b)), // BigInt as string in JSON
    }
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

// ============================================================================
// Global Functions
// ============================================================================

fn builtin_parse_int(args: &[Value]) -> Value {
    if let Some(val) = args.first() {
        let s = val.to_string();
        if let Ok(n) = s.trim().parse::<i64>() {
            return Value::Number(n as f64);
        }
    }
    Value::Number(f64::NAN)
}

fn builtin_parse_float(args: &[Value]) -> Value {
    if let Some(val) = args.first() {
        let s = val.to_string();
        if let Ok(n) = s.trim().parse::<f64>() {
            return Value::Number(n);
        }
    }
    Value::Number(f64::NAN)
}

fn builtin_is_nan(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Boolean(n.is_nan())
    } else {
        Value::Boolean(true)
    }
}

fn builtin_is_finite(args: &[Value]) -> Value {
    if let Some(Value::Number(n)) = args.first() {
        Value::Boolean(n.is_finite())
    } else {
        Value::Boolean(false)
    }
}

// ============================================================================
// Promise Methods
// ============================================================================

use crate::value::{PromiseState, PromiseValue};

/// Promise.resolve - creates a resolved promise
fn builtin_promise_resolve(args: &[Value]) -> Value {
    let value = args.first().cloned().unwrap_or(Value::Undefined);

    // If the value is already a promise, return it
    if let Value::Promise(_) = &value {
        return value;
    }

    // Create a fulfilled promise
    Value::Promise(Box::new(PromiseValue {
        state: PromiseState::Fulfilled(Box::new(value)),
    }))
}

/// Promise.reject - creates a rejected promise
fn builtin_promise_reject(args: &[Value]) -> Value {
    let reason = args.first().cloned().unwrap_or(Value::Undefined);

    Value::Promise(Box::new(PromiseValue {
        state: PromiseState::Rejected(Box::new(reason)),
    }))
}

/// Promise.all - resolves when all promises resolve, rejects when any rejects
fn builtin_promise_all(args: &[Value]) -> Value {
    let promises = match args.first() {
        Some(Value::Array(arr)) => arr,
        Some(other) => {
            throw_type_error(format!(
                "Promise.all requires an iterable (received {})",
                other.type_name()
            ));
            return builtin_promise_reject(&[Value::String(
                "Promise.all requires an array".to_string(),
            )]);
        }
        None => {
            throw_type_error("Promise.all requires an argument");
            return builtin_promise_reject(&[Value::String(
                "Promise.all requires an array".to_string(),
            )]);
        }
    };

    let mut results = Vec::with_capacity(promises.len());

    for promise in promises {
        match promise {
            Value::Promise(p) => match &p.state {
                PromiseState::Fulfilled(value) => {
                    results.push((**value).clone());
                }
                PromiseState::Rejected(reason) => {
                    // If any promise rejects, return a rejected promise
                    return Value::Promise(Box::new(PromiseValue {
                        state: PromiseState::Rejected(reason.clone()),
                    }));
                }
                PromiseState::Pending => {
                    // For pending promises, we return a pending promise
                    // In a real implementation, this would be async
                    return Value::Promise(Box::new(PromiseValue {
                        state: PromiseState::Pending,
                    }));
                }
            },
            // Non-promise values are treated as resolved
            other => {
                results.push(other.clone());
            }
        }
    }

    // All promises resolved
    Value::Promise(Box::new(PromiseValue {
        state: PromiseState::Fulfilled(Box::new(Value::Array(results))),
    }))
}

/// Promise.race - settles with the first settled promise
fn builtin_promise_race(args: &[Value]) -> Value {
    let promises = match args.first() {
        Some(Value::Array(arr)) => arr,
        Some(other) => {
            throw_type_error(format!(
                "Promise.race requires an iterable (received {})",
                other.type_name()
            ));
            return builtin_promise_reject(&[Value::String(
                "Promise.race requires an array".to_string(),
            )]);
        }
        None => {
            throw_type_error("Promise.race requires an argument");
            return builtin_promise_reject(&[Value::String(
                "Promise.race requires an array".to_string(),
            )]);
        }
    };

    // Find the first settled promise
    for promise in promises {
        match promise {
            Value::Promise(p) => match &p.state {
                PromiseState::Fulfilled(value) => {
                    return Value::Promise(Box::new(PromiseValue {
                        state: PromiseState::Fulfilled(value.clone()),
                    }));
                }
                PromiseState::Rejected(reason) => {
                    return Value::Promise(Box::new(PromiseValue {
                        state: PromiseState::Rejected(reason.clone()),
                    }));
                }
                PromiseState::Pending => {
                    // Continue checking other promises
                }
            },
            // Non-promise values are treated as immediately resolved
            other => {
                return Value::Promise(Box::new(PromiseValue {
                    state: PromiseState::Fulfilled(Box::new(other.clone())),
                }));
            }
        }
    }

    // All promises are pending
    Value::Promise(Box::new(PromiseValue {
        state: PromiseState::Pending,
    }))
}

/// Promise.any - resolves with first fulfilled, rejects with AggregateError if all reject
fn builtin_promise_any(args: &[Value]) -> Value {
    let promises = match args.first() {
        Some(Value::Array(arr)) => arr,
        Some(other) => {
            throw_type_error(format!(
                "Promise.any requires an iterable (received {})",
                other.type_name()
            ));
            return builtin_promise_reject(&[Value::String(
                "Promise.any requires an array".to_string(),
            )]);
        }
        None => {
            throw_type_error("Promise.any requires an argument");
            return builtin_promise_reject(&[Value::String(
                "Promise.any requires an array".to_string(),
            )]);
        }
    };

    let mut errors = Vec::new();
    let mut has_pending = false;

    for promise in promises {
        match promise {
            Value::Promise(p) => match &p.state {
                PromiseState::Fulfilled(value) => {
                    // First fulfilled promise wins
                    return Value::Promise(Box::new(PromiseValue {
                        state: PromiseState::Fulfilled(value.clone()),
                    }));
                }
                PromiseState::Rejected(reason) => {
                    errors.push((**reason).clone());
                }
                PromiseState::Pending => {
                    has_pending = true;
                }
            },
            // Non-promise values are treated as immediately resolved
            other => {
                return Value::Promise(Box::new(PromiseValue {
                    state: PromiseState::Fulfilled(Box::new(other.clone())),
                }));
            }
        }
    }

    if has_pending {
        // Some promises are still pending
        return Value::Promise(Box::new(PromiseValue {
            state: PromiseState::Pending,
        }));
    }

    // All promises rejected - create AggregateError
    // For simplicity, we use an array of errors
    Value::Promise(Box::new(PromiseValue {
        state: PromiseState::Rejected(Box::new(Value::Array(errors))),
    }))
}

/// Promise.allSettled - resolves with all settlement results
fn builtin_promise_all_settled(args: &[Value]) -> Value {
    use crate::value::object::Object;

    let promises = match args.first() {
        Some(Value::Array(arr)) => arr,
        Some(other) => {
            throw_type_error(format!(
                "Promise.allSettled requires an iterable (received {})",
                other.type_name()
            ));
            return builtin_promise_reject(&[Value::String(
                "Promise.allSettled requires an array".to_string(),
            )]);
        }
        None => {
            throw_type_error("Promise.allSettled requires an argument");
            return builtin_promise_reject(&[Value::String(
                "Promise.allSettled requires an array".to_string(),
            )]);
        }
    };

    let mut results = Vec::with_capacity(promises.len());
    let mut has_pending = false;

    for promise in promises {
        match promise {
            Value::Promise(p) => match &p.state {
                PromiseState::Fulfilled(value) => {
                    let mut result = Object::new();
                    result.set("status".to_string(), Value::String("fulfilled".to_string()));
                    result.set("value".to_string(), (**value).clone());
                    results.push(Value::Object(result));
                }
                PromiseState::Rejected(reason) => {
                    let mut result = Object::new();
                    result.set("status".to_string(), Value::String("rejected".to_string()));
                    result.set("reason".to_string(), (**reason).clone());
                    results.push(Value::Object(result));
                }
                PromiseState::Pending => {
                    has_pending = true;
                }
            },
            // Non-promise values are treated as immediately resolved
            other => {
                let mut result = Object::new();
                result.set("status".to_string(), Value::String("fulfilled".to_string()));
                result.set("value".to_string(), other.clone());
                results.push(Value::Object(result));
            }
        }
    }

    if has_pending {
        // Some promises are still pending
        return Value::Promise(Box::new(PromiseValue {
            state: PromiseState::Pending,
        }));
    }

    // All promises settled
    Value::Promise(Box::new(PromiseValue {
        state: PromiseState::Fulfilled(Box::new(Value::Array(results))),
    }))
}

// ============================================================================
// Process Methods
// ============================================================================

/// process.memoryUsage - returns memory usage statistics
/// Returns an object with heapTotal, heapUsed, rss, external, arrayBuffers
fn builtin_process_memory_usage(_args: &[Value]) -> Value {
    use crate::compiler::codegen::get_runtime_memory_usage;
    use crate::value::object::Object;

    let (heap_used, heap_total, _object_count) = get_runtime_memory_usage();

    // Estimate RSS as heap_total + some overhead
    let rss = heap_total + (heap_total / 10);

    let mut result = Object::new();
    result.set("rss".to_string(), Value::Number(rss as f64));
    result.set("heapTotal".to_string(), Value::Number(heap_total as f64));
    result.set("heapUsed".to_string(), Value::Number(heap_used as f64));
    result.set("external".to_string(), Value::Number(0.0));
    result.set("arrayBuffers".to_string(), Value::Number(0.0));

    Value::Object(result)
}

// ============================================================================
// DX Global Object Methods
// ============================================================================

/// dx.features - returns an object with supported ECMAScript features
///
/// Returns an object with boolean values for each feature key.
/// Required keys per Property 16: es2015, es2016, es2017, es2018, es2019,
/// es2020, es2021, es2022, typescript
///
/// # Example
/// ```javascript
/// if (dx.features.es2022) {
///     // Use ES2022 features
/// }
/// if (dx.features.typescript) {
///     // TypeScript is supported
/// }
/// ```
fn builtin_dx_features(_args: &[Value]) -> Value {
    use crate::features::DxFeatures;
    use crate::value::object::Object;

    let features = DxFeatures::current();
    let feature_map = features.to_map();

    let mut result = Object::new();
    for (key, value) in feature_map {
        result.set(key, Value::Boolean(value));
    }

    Value::Object(result)
}

/// dx.version - returns the DX runtime version string
///
/// Returns the semantic version of the DX runtime (e.g., "0.0.1")
///
/// # Example
/// ```javascript
/// console.log(dx.version); // "0.0.1"
/// ```
fn builtin_dx_version(_args: &[Value]) -> Value {
    use crate::features::DxGlobal;

    let dx = DxGlobal::new();
    Value::String(dx.version.to_string())
}
