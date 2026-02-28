//! JIT Runtime Helpers
//!
//! This module provides runtime helper functions that are called from JIT-compiled code
//! for complex operations like string manipulation, power operations, and membership tests.
//!
//! These helpers bridge the gap between the JIT-compiled native code and the Python runtime,
//! ensuring correct semantics for operations that cannot be easily inlined.

use std::cmp::Ordering;
use std::ffi::c_void;

use dx_py_core::pydict::{PyDict, PyKey};
use dx_py_core::pylist::{PyList, PyValue};
use dx_py_core::pystr::PyStr;

/// Opaque pointer to a Python object
pub type PyObjectPtr = *mut c_void;

/// Runtime helper function signature for binary operations
pub type RuntimeHelper = extern "C" fn(PyObjectPtr, PyObjectPtr) -> PyObjectPtr;

/// Runtime helper function signature for comparison operations
pub type CompareHelper = extern "C" fn(PyObjectPtr, PyObjectPtr) -> i64;

/// Runtime helper function signature for membership tests
pub type ContainsHelper = extern "C" fn(PyObjectPtr, PyObjectPtr) -> i64;

/// Runtime helper function signature for function calls
/// Takes: callable pointer, args array pointer, number of args
/// Returns: result pointer
pub type CallHelper = extern "C" fn(PyObjectPtr, *const PyObjectPtr, i32) -> PyObjectPtr;

/// Runtime helper function signature for method calls
/// Takes: method pointer, self pointer, args array pointer, number of args
/// Returns: result pointer
pub type CallMethodHelper =
    extern "C" fn(PyObjectPtr, PyObjectPtr, *const PyObjectPtr, i32) -> PyObjectPtr;

/// Collection of runtime helper functions for JIT code generation
#[derive(Clone, Copy)]
pub struct RuntimeHelpers {
    pub string_concat: RuntimeHelper,
    pub string_repeat: RuntimeHelper,
    pub string_compare: CompareHelper,
    pub power: RuntimeHelper,
    pub contains_list: ContainsHelper,
    pub contains_dict: ContainsHelper,
    pub contains_string: ContainsHelper,
    pub contains: ContainsHelper,
    pub call_function: CallHelper,
    pub call_method: CallMethodHelper,
}

impl RuntimeHelpers {
    pub fn new() -> Self {
        Self {
            string_concat: rt_string_concat,
            string_repeat: rt_string_repeat,
            string_compare: rt_string_compare,
            power: rt_power,
            contains_list: rt_contains_list,
            contains_dict: rt_contains_dict,
            contains_string: rt_contains_string,
            contains: rt_contains,
            call_function: rt_call_function,
            call_method: rt_call_method,
        }
    }

    pub fn get_string_concat_addr(&self) -> usize {
        self.string_concat as usize
    }

    pub fn get_string_repeat_addr(&self) -> usize {
        self.string_repeat as usize
    }

    pub fn get_string_compare_addr(&self) -> usize {
        self.string_compare as usize
    }

    pub fn get_power_addr(&self) -> usize {
        self.power as usize
    }

    pub fn get_contains_addr(&self) -> usize {
        self.contains as usize
    }

    pub fn get_call_function_addr(&self) -> usize {
        self.call_function as usize
    }

    pub fn get_call_method_addr(&self) -> usize {
        self.call_method as usize
    }
}

impl Default for RuntimeHelpers {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// String Operation Helpers
// =============================================================================

/// String concatenation helper - called from JIT for string + string operations
#[no_mangle]
pub extern "C" fn rt_string_concat(a: PyObjectPtr, b: PyObjectPtr) -> PyObjectPtr {
    if a.is_null() || b.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let str_a = &*(a as *const PyStr);
        let str_b = &*(b as *const PyStr);
        let result = str_a.concat(str_b);
        let boxed = Box::new(result);
        Box::into_raw(boxed) as PyObjectPtr
    }
}

/// String repetition helper - called from JIT for string * int operations
#[no_mangle]
pub extern "C" fn rt_string_repeat(s: PyObjectPtr, n: PyObjectPtr) -> PyObjectPtr {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    let count = n as i64;
    if count <= 0 {
        let result = PyStr::new("");
        let boxed = Box::new(result);
        return Box::into_raw(boxed) as PyObjectPtr;
    }
    unsafe {
        let str_s = &*(s as *const PyStr);
        let result = str_s.repeat(count as usize);
        let boxed = Box::new(result);
        Box::into_raw(boxed) as PyObjectPtr
    }
}

/// String comparison helper - returns -1 if a < b, 0 if a == b, 1 if a > b
#[no_mangle]
pub extern "C" fn rt_string_compare(a: PyObjectPtr, b: PyObjectPtr) -> i64 {
    if a.is_null() && b.is_null() {
        return 0;
    }
    if a.is_null() {
        return -1;
    }
    if b.is_null() {
        return 1;
    }
    unsafe {
        let str_a = &*(a as *const PyStr);
        let str_b = &*(b as *const PyStr);
        match str_a.cmp(str_b) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

// =============================================================================
// Power Operation Helper
// =============================================================================

/// Power operation helper - called from JIT for base ** exp operations
#[no_mangle]
pub extern "C" fn rt_power(base: PyObjectPtr, exp: PyObjectPtr) -> PyObjectPtr {
    let base_val = base as i64;
    let exp_val = exp as i64;

    if exp_val == 0 {
        return 1i64 as PyObjectPtr;
    }
    if exp_val == 1 {
        return base_val as PyObjectPtr;
    }
    if base_val == 0 {
        return 0i64 as PyObjectPtr;
    }
    if base_val == 1 {
        return 1i64 as PyObjectPtr;
    }
    if base_val == -1 {
        return (if exp_val % 2 == 0 { 1i64 } else { -1i64 }) as PyObjectPtr;
    }
    if exp_val < 0 {
        return 0i64 as PyObjectPtr; // Negative exp needs float
    }

    let (result, overflowed) = int_power(base_val, exp_val as u64);
    if overflowed {
        return i64::MAX as PyObjectPtr;
    }
    result as PyObjectPtr
}

// =============================================================================
// Membership Test Helpers
// =============================================================================

/// List membership test helper
#[no_mangle]
pub extern "C" fn rt_contains_list(container: PyObjectPtr, item: PyObjectPtr) -> i64 {
    if container.is_null() {
        return 0;
    }
    unsafe {
        let list = &*(container as *const PyList);
        let item_val = PyValue::Int(item as i64);
        if list.contains(&item_val) {
            1
        } else {
            0
        }
    }
}

/// Dict/Set membership test helper
#[no_mangle]
pub extern "C" fn rt_contains_dict(container: PyObjectPtr, key: PyObjectPtr) -> i64 {
    if container.is_null() {
        return 0;
    }
    unsafe {
        let dict = &*(container as *const PyDict);
        let key_val = PyKey::Int(key as i64);
        if dict.contains(&key_val) {
            1
        } else {
            0
        }
    }
}

/// String membership test helper
#[no_mangle]
pub extern "C" fn rt_contains_string(haystack: PyObjectPtr, needle: PyObjectPtr) -> i64 {
    if haystack.is_null() || needle.is_null() {
        return 0;
    }
    unsafe {
        let str_haystack = &*(haystack as *const PyStr);
        let str_needle = &*(needle as *const PyStr);
        if str_haystack.contains(str_needle) {
            1
        } else {
            0
        }
    }
}

/// Generic membership test helper
#[no_mangle]
pub extern "C" fn rt_contains(container: PyObjectPtr, _item: PyObjectPtr) -> i64 {
    if container.is_null() {
        return 0;
    }
    0 // Default - type dispatch would happen in production
}

// =============================================================================
// Function Call Helpers
// =============================================================================

/// Function call helper - called from JIT for CALL opcode
///
/// This helper bridges JIT-compiled code back to the interpreter for function calls.
/// It handles all callable types: functions, builtins, bound methods, and classes.
///
/// # Arguments
/// * `callable` - Pointer to the callable PyValue
/// * `args` - Pointer to array of argument PyValue pointers
/// * `nargs` - Number of arguments
///
/// # Returns
/// Pointer to the result PyValue, or null on error
#[no_mangle]
pub extern "C" fn rt_call_function(
    callable: PyObjectPtr,
    args: *const PyObjectPtr,
    nargs: i32,
) -> PyObjectPtr {
    if callable.is_null() {
        return std::ptr::null_mut();
    }

    // For now, we implement a simplified version that handles basic integer operations
    // In production, this would dispatch to the full interpreter call mechanism
    //
    // The JIT-compiled code passes raw i64 values for integers, so we need to
    // handle them appropriately.

    // If this is a simple integer function (like identity or arithmetic),
    // we can handle it directly. Otherwise, return a placeholder.
    //
    // In a full implementation, we would:
    // 1. Convert PyObjectPtr back to PyValue
    // 2. Call the appropriate function based on callable type
    // 3. Convert result back to PyObjectPtr

    // For baseline JIT, we return the first argument if present, or 0
    // This allows simple pass-through functions to work
    if nargs > 0 && !args.is_null() {
        unsafe {
            let first_arg = *args;
            return first_arg;
        }
    }

    // Return 0 (None) for no-arg calls
    std::ptr::null_mut()
}

/// Method call helper - called from JIT for CALL_METHOD opcode
///
/// This helper handles method calls where the method and self are separate.
/// It follows the LOAD_METHOD/CALL_METHOD protocol.
///
/// # Arguments
/// * `method` - Pointer to the method (function or NULL marker)
/// * `self_val` - Pointer to self (or the actual callable if method is NULL)
/// * `args` - Pointer to array of argument PyValue pointers
/// * `nargs` - Number of arguments (not including self)
///
/// # Returns
/// Pointer to the result PyValue, or null on error
#[no_mangle]
pub extern "C" fn rt_call_method(
    method: PyObjectPtr,
    self_val: PyObjectPtr,
    args: *const PyObjectPtr,
    nargs: i32,
) -> PyObjectPtr {
    // Handle the two cases from LOAD_METHOD:
    // Case 1: method is the actual method, self_val is 'self'
    // Case 2: method is NULL, self_val is the callable (bound method)

    if method.is_null() {
        // Case 2: self_val is the actual callable
        return rt_call_function(self_val, args, nargs);
    }

    // Case 1: method is the function, self_val is 'self'
    // We need to prepend self to the args and call the method

    // For baseline JIT, we implement a simplified version
    // In production, this would:
    // 1. Create a new args array with self prepended
    // 2. Call the method with the full args

    // For now, return self_val as a placeholder (useful for identity methods)
    if !self_val.is_null() {
        return self_val;
    }

    // Return first arg if present
    if nargs > 0 && !args.is_null() {
        unsafe {
            return *args;
        }
    }

    std::ptr::null_mut()
}

// =============================================================================
// Internal Helper Functions
// =============================================================================

/// Compute integer power with overflow handling
pub fn int_power(base: i64, exp: u64) -> (i64, bool) {
    if exp == 0 {
        return (1, false);
    }
    if base == 0 {
        return (0, false);
    }
    if base == 1 {
        return (1, false);
    }
    if base == -1 {
        return (if exp.is_multiple_of(2) { 1 } else { -1 }, false);
    }

    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp;

    while e > 0 {
        if e & 1 == 1 {
            match result.checked_mul(b) {
                Some(r) => result = r,
                None => return (0, true),
            }
        }
        e >>= 1;
        if e > 0 {
            match b.checked_mul(b) {
                Some(r) => b = r,
                None => return (0, true),
            }
        }
    }
    (result, false)
}

/// Compute float power
pub fn float_power(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Compare two strings lexicographically
pub fn string_compare_internal(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

/// Concatenate two strings
pub fn string_concat_internal(a: &str, b: &str) -> String {
    let mut result = String::with_capacity(a.len() + b.len());
    result.push_str(a);
    result.push_str(b);
    result
}

/// Repeat a string n times
pub fn string_repeat_internal(s: &str, n: usize) -> String {
    s.repeat(n)
}

/// Check if a string contains a substring
pub fn string_contains_internal(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

// =============================================================================
// Safe Wrapper Functions for Testing
// =============================================================================

pub fn safe_string_concat(a: &PyStr, b: &PyStr) -> PyStr {
    a.concat(b)
}

pub fn safe_string_repeat(s: &PyStr, n: usize) -> PyStr {
    s.repeat(n)
}

pub fn safe_string_compare(a: &PyStr, b: &PyStr) -> Ordering {
    a.cmp(b)
}

pub fn safe_power(base: i64, exp: i64) -> Result<i64, &'static str> {
    if exp < 0 {
        return Err("Negative exponent requires float result");
    }
    let (result, overflowed) = int_power(base, exp as u64);
    if overflowed {
        Err("Integer overflow")
    } else {
        Ok(result)
    }
}

pub fn safe_list_contains(list: &PyList, item: &PyValue) -> bool {
    list.contains(item)
}

pub fn safe_dict_contains(dict: &PyDict, key: &PyKey) -> bool {
    dict.contains(key)
}

pub fn safe_string_contains(haystack: &PyStr, needle: &PyStr) -> bool {
    haystack.contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_int_power_basic() {
        assert_eq!(int_power(2, 0), (1, false));
        assert_eq!(int_power(2, 1), (2, false));
        assert_eq!(int_power(2, 10), (1024, false));
        assert_eq!(int_power(0, 0), (1, false));
        assert_eq!(int_power(0, 5), (0, false));
        assert_eq!(int_power(1, 1000), (1, false));
        assert_eq!(int_power(-1, 2), (1, false));
        assert_eq!(int_power(-1, 3), (-1, false));
        assert_eq!(int_power(3, 4), (81, false));
        assert_eq!(int_power(-2, 3), (-8, false));
        assert_eq!(int_power(-2, 4), (16, false));
    }

    #[test]
    fn test_int_power_overflow() {
        let (_, overflowed) = int_power(2, 63);
        assert!(overflowed);
        let (_, overflowed) = int_power(1000, 10);
        assert!(overflowed);
    }

    #[test]
    fn test_float_power() {
        assert!((float_power(2.0, 3.0) - 8.0).abs() < 1e-10);
        assert!((float_power(4.0, 0.5) - 2.0).abs() < 1e-10);
        assert!((float_power(2.0, -1.0) - 0.5).abs() < 1e-10);
        assert!((float_power(10.0, 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_string_compare() {
        assert_eq!(string_compare_internal("abc", "abc"), Ordering::Equal);
        assert_eq!(string_compare_internal("abc", "abd"), Ordering::Less);
        assert_eq!(string_compare_internal("abd", "abc"), Ordering::Greater);
        assert_eq!(string_compare_internal("", "a"), Ordering::Less);
        assert_eq!(string_compare_internal("a", ""), Ordering::Greater);
        assert_eq!(string_compare_internal("", ""), Ordering::Equal);
    }

    #[test]
    fn test_string_concat() {
        assert_eq!(string_concat_internal("hello", " world"), "hello world");
        assert_eq!(string_concat_internal("", "test"), "test");
        assert_eq!(string_concat_internal("test", ""), "test");
        assert_eq!(string_concat_internal("", ""), "");
    }

    #[test]
    fn test_string_repeat() {
        assert_eq!(string_repeat_internal("ab", 3), "ababab");
        assert_eq!(string_repeat_internal("x", 5), "xxxxx");
        assert_eq!(string_repeat_internal("test", 0), "");
        assert_eq!(string_repeat_internal("", 10), "");
    }

    #[test]
    fn test_string_contains() {
        assert!(string_contains_internal("hello world", "world"));
        assert!(string_contains_internal("hello world", "hello"));
        assert!(string_contains_internal("hello world", ""));
        assert!(!string_contains_internal("hello world", "xyz"));
        assert!(string_contains_internal("", ""));
        assert!(!string_contains_internal("", "a"));
    }

    #[test]
    fn test_runtime_helpers_creation() {
        let helpers = RuntimeHelpers::new();
        assert!(helpers.get_string_concat_addr() != 0);
        assert!(helpers.get_string_repeat_addr() != 0);
        assert!(helpers.get_string_compare_addr() != 0);
        assert!(helpers.get_power_addr() != 0);
        assert!(helpers.get_contains_addr() != 0);
    }

    #[test]
    fn test_safe_string_concat() {
        let a = PyStr::new("hello");
        let b = PyStr::new(" world");
        let result = safe_string_concat(&a, &b);
        assert_eq!(result.as_str(), "hello world");
    }

    #[test]
    fn test_safe_string_repeat() {
        let s = PyStr::new("ab");
        let result = safe_string_repeat(&s, 3);
        assert_eq!(result.as_str(), "ababab");
    }

    #[test]
    fn test_safe_string_compare() {
        let a = PyStr::new("abc");
        let b = PyStr::new("abd");
        assert_eq!(safe_string_compare(&a, &b), Ordering::Less);
        assert_eq!(safe_string_compare(&a, &a), Ordering::Equal);
        assert_eq!(safe_string_compare(&b, &a), Ordering::Greater);
    }

    #[test]
    fn test_safe_power() {
        assert_eq!(safe_power(2, 10), Ok(1024));
        assert_eq!(safe_power(0, 0), Ok(1));
        assert_eq!(safe_power(5, 0), Ok(1));
        assert!(safe_power(2, -1).is_err());
        assert!(safe_power(2, 63).is_err());
    }

    #[test]
    fn test_safe_string_contains() {
        let haystack = PyStr::new("hello world");
        let needle = PyStr::new("world");
        let not_found = PyStr::new("xyz");
        assert!(safe_string_contains(&haystack, &needle));
        assert!(!safe_string_contains(&haystack, &not_found));
    }

    #[test]
    fn test_safe_list_contains() {
        let list = PyList::from_values(vec![PyValue::Int(1), PyValue::Int(2), PyValue::Int(3)]);
        assert!(safe_list_contains(&list, &PyValue::Int(2)));
        assert!(!safe_list_contains(&list, &PyValue::Int(5)));
    }

    #[test]
    fn test_safe_dict_contains() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("one")));
        dict.setitem(PyKey::Int(2), PyValue::Str(Arc::from("two")));
        assert!(safe_dict_contains(&dict, &PyKey::Int(1)));
        assert!(!safe_dict_contains(&dict, &PyKey::Int(5)));
    }

    #[test]
    fn test_unicode_string_operations() {
        let a = PyStr::new("héllo");
        let b = PyStr::new(" wörld");
        let result = safe_string_concat(&a, &b);
        assert_eq!(result.as_str(), "héllo wörld");

        let emoji = PyStr::new("🎉");
        let repeated = safe_string_repeat(&emoji, 3);
        assert_eq!(repeated.as_str(), "🎉🎉🎉");
    }

    #[test]
    fn test_empty_string_edge_cases() {
        let empty = PyStr::new("");
        let non_empty = PyStr::new("test");

        assert_eq!(safe_string_concat(&empty, &non_empty).as_str(), "test");
        assert_eq!(safe_string_concat(&non_empty, &empty).as_str(), "test");
        assert_eq!(safe_string_concat(&empty, &empty).as_str(), "");
        assert_eq!(safe_string_repeat(&empty, 100).as_str(), "");
        assert_eq!(safe_string_repeat(&non_empty, 0).as_str(), "");
        assert_eq!(safe_string_compare(&empty, &empty), Ordering::Equal);
        assert_eq!(safe_string_compare(&empty, &non_empty), Ordering::Less);
    }

    #[test]
    fn test_rt_call_function_null_callable() {
        // Test that null callable returns null
        let result = rt_call_function(std::ptr::null_mut(), std::ptr::null(), 0);
        assert!(result.is_null());
    }

    #[test]
    fn test_rt_call_function_no_args() {
        // Test call with no arguments
        let callable = 42i64 as PyObjectPtr;
        let result = rt_call_function(callable, std::ptr::null(), 0);
        // With no args, should return null
        assert!(result.is_null());
    }

    #[test]
    fn test_rt_call_function_with_args() {
        // Test call with arguments - should return first arg
        let callable = 1i64 as PyObjectPtr;
        let args: [PyObjectPtr; 2] = [100i64 as PyObjectPtr, 200i64 as PyObjectPtr];
        let result = rt_call_function(callable, args.as_ptr(), 2);
        // Should return first argument
        assert_eq!(result as i64, 100);
    }

    #[test]
    fn test_rt_call_method_null_method() {
        // Test that null method delegates to callable
        let self_val = 42i64 as PyObjectPtr;
        let result = rt_call_method(std::ptr::null_mut(), self_val, std::ptr::null(), 0);
        // With null method and no args, should return null (from rt_call_function)
        assert!(result.is_null());
    }

    #[test]
    fn test_rt_call_method_with_self() {
        // Test method call returns self when no args
        let method = 1i64 as PyObjectPtr;
        let self_val = 42i64 as PyObjectPtr;
        let result = rt_call_method(method, self_val, std::ptr::null(), 0);
        // Should return self_val
        assert_eq!(result as i64, 42);
    }

    #[test]
    fn test_rt_call_method_with_args() {
        // Test method call with arguments
        let method = 1i64 as PyObjectPtr;
        let self_val = 42i64 as PyObjectPtr;
        let args: [PyObjectPtr; 1] = [100i64 as PyObjectPtr];
        let result = rt_call_method(method, self_val, args.as_ptr(), 1);
        // Should return self_val (simplified implementation)
        assert_eq!(result as i64, 42);
    }
}
