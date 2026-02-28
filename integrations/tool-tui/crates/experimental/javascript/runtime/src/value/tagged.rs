//! NaN-boxed value representation for efficient JavaScript values
//!
//! This module implements a NaN-boxing technique for representing JavaScript values
//! in a single 64-bit word. This provides:
//! - Efficient storage (8 bytes per value)
//! - Fast type checking (bit operations)
//! - No heap allocation for primitives
//!
//! Layout (64 bits):
//! - Float: Standard IEEE 754 double (when not a signaling NaN with our tag bits)
//! - Tagged values use the negative NaN space (sign bit = 1):
//!   - Pointer: 0xFFF8_xxxx_xxxx_xxxx (negative quiet NaN + 48-bit pointer)
//!   - Integer: 0xFFF9_xxxx_xxxx_xxxx (32-bit signed integer)
//!   - Boolean: 0xFFFA_0000_0000_000x (x = 0 or 1)
//!   - Null: 0xFFFB_0000_0000_0000
//!   - Undefined: 0xFFFC_0000_0000_0000
//!   - Symbol: 0xFFFD_xxxx_xxxx_xxxx (symbol ID)
//!   - BigInt: 0xFFFE_xxxx_xxxx_xxxx (pointer to BigInt)

use std::fmt;
use std::ptr::NonNull;

/// Tag bits for different value types (stored in high 16 bits)
/// We use the NEGATIVE NaN space (sign bit = 1) to avoid conflicts with regular NaN
const TAG_MASK: u64 = 0xFFFF_0000_0000_0000;
/// Mask for heap object subtypes (includes subtype bits in bits 48-63)
const SUBTYPE_MASK: u64 = 0xFFFF_FFFF_0000_0000;
const TAG_POINTER: u64 = 0xFFF8_0000_0000_0000;
const TAG_STRING: u64 = 0xFFF8_0000_0000_0000; // Strings are pointers
const TAG_OBJECT: u64 = 0xFFF8_0001_0000_0000; // Objects are pointers with subtype
const TAG_ARRAY: u64 = 0xFFF8_0002_0000_0000; // Arrays are pointers with subtype
const TAG_FUNCTION: u64 = 0xFFF8_0003_0000_0000; // Functions are pointers with subtype
const TAG_INTEGER: u64 = 0xFFF9_0000_0000_0000;
const TAG_BOOLEAN: u64 = 0xFFFA_0000_0000_0000;
const TAG_NULL: u64 = 0xFFFB_0000_0000_0000;
const TAG_UNDEFINED: u64 = 0xFFFC_0000_0000_0000;
const TAG_SYMBOL: u64 = 0xFFFD_0000_0000_0000;
const TAG_BIGINT: u64 = 0xFFFE_0000_0000_0000;

/// Mask for extracting 48-bit payload (pointer or value)
const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

/// Mask for extracting 32-bit integer payload
const INT_PAYLOAD_MASK: u64 = 0x0000_0000_FFFF_FFFF;

/// Check if high bits indicate a tagged value (negative NaN space)
/// Tagged values have bits 0xFFF8 through 0xFFFE in the high 16 bits
const TAG_MIN: u64 = 0xFFF8_0000_0000_0000;
const TAG_MAX: u64 = 0xFFFE_FFFF_FFFF_FFFF;

/// NaN-boxed JavaScript value (64-bit)
///
/// This is the primary value representation used throughout the runtime.
/// It efficiently stores all JavaScript primitive types and pointers to
/// heap-allocated objects in a single 64-bit word.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TaggedValue(u64);

/// Type tag for heap objects
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HeapObjectType {
    String = 0,
    Object = 1,
    Array = 2,
    Function = 3,
    Promise = 4,
    RegExp = 5,
    Date = 6,
    Map = 7,
    Set = 8,
    WeakMap = 9,
    WeakSet = 10,
    ArrayBuffer = 11,
    TypedArray = 12,
    DataView = 13,
    Error = 14,
    BigInt = 15,
}

impl TaggedValue {
    // ==================== Constructors ====================

    /// Create from f64 number
    #[inline]
    pub const fn from_f64(n: f64) -> Self {
        Self(n.to_bits())
    }

    /// Create from i32 integer (stored inline, no heap allocation)
    #[inline]
    pub const fn from_i32(n: i32) -> Self {
        // Store as unsigned to preserve bit pattern
        Self(TAG_INTEGER | (n as u32 as u64))
    }

    /// Create from bool
    #[inline]
    pub const fn from_bool(b: bool) -> Self {
        Self(TAG_BOOLEAN | (b as u64))
    }

    /// Create null value
    #[inline]
    pub const fn null() -> Self {
        Self(TAG_NULL)
    }

    /// Create undefined value
    #[inline]
    pub const fn undefined() -> Self {
        Self(TAG_UNDEFINED)
    }

    /// Create from raw pointer with object type tag
    #[inline]
    pub fn from_ptr(ptr: *const u8, obj_type: HeapObjectType) -> Self {
        let type_bits = (obj_type as u64) << 48;
        Self(TAG_POINTER | type_bits | (ptr as u64 & PAYLOAD_MASK))
    }

    /// Create from string pointer
    #[inline]
    pub fn from_string_ptr(ptr: *const u8) -> Self {
        Self(TAG_STRING | (ptr as u64 & PAYLOAD_MASK))
    }

    /// Create from object pointer
    #[inline]
    pub fn from_object_ptr(ptr: *const u8) -> Self {
        Self(TAG_OBJECT | (ptr as u64 & PAYLOAD_MASK))
    }

    /// Create from array pointer
    #[inline]
    pub fn from_array_ptr(ptr: *const u8) -> Self {
        Self(TAG_ARRAY | (ptr as u64 & PAYLOAD_MASK))
    }

    /// Create from function pointer
    #[inline]
    pub fn from_function_ptr(ptr: *const u8) -> Self {
        Self(TAG_FUNCTION | (ptr as u64 & PAYLOAD_MASK))
    }

    /// Create from symbol ID
    #[inline]
    pub const fn from_symbol(id: u32) -> Self {
        Self(TAG_SYMBOL | (id as u64))
    }

    /// Create from BigInt pointer
    #[inline]
    pub fn from_bigint_ptr(ptr: *const u8) -> Self {
        Self(TAG_BIGINT | (ptr as u64 & PAYLOAD_MASK))
    }

    // ==================== Type Checking ====================

    /// Check if this is a number (f64)
    #[inline]
    pub fn is_number(&self) -> bool {
        // A value is a number if it's not one of our tagged values
        // Our tags use the NEGATIVE NaN space (sign bit = 1): 0xFFF8 through 0xFFFE
        // Regular floats (including positive NaN 0x7FF8...) are numbers
        self.0 < TAG_MIN || self.0 > TAG_MAX
    }

    /// Check if this is an integer (i32 stored inline)
    #[inline]
    pub fn is_integer(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_INTEGER
    }

    /// Check if this is a string
    #[inline]
    pub fn is_string(&self) -> bool {
        // Strings have TAG_POINTER (0xFFF8) in high 16 bits and zero subtype bits
        // Use SUBTYPE_MASK to check both the tag and subtype are exactly TAG_STRING
        (self.0 & SUBTYPE_MASK) == TAG_STRING
    }

    /// Check if this is an object (not array, not function)
    #[inline]
    pub fn is_object(&self) -> bool {
        (self.0 & SUBTYPE_MASK) == TAG_OBJECT
    }

    /// Check if this is an array
    #[inline]
    pub fn is_array(&self) -> bool {
        (self.0 & SUBTYPE_MASK) == TAG_ARRAY
    }

    /// Check if this is a function
    #[inline]
    pub fn is_function(&self) -> bool {
        (self.0 & SUBTYPE_MASK) == TAG_FUNCTION
    }

    /// Check if this is a boolean
    #[inline]
    pub fn is_boolean(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_BOOLEAN
    }

    /// Check if this is null
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == TAG_NULL
    }

    /// Check if this is undefined
    #[inline]
    pub fn is_undefined(&self) -> bool {
        self.0 == TAG_UNDEFINED
    }

    /// Check if this is a symbol
    #[inline]
    pub fn is_symbol(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_SYMBOL
    }

    /// Check if this is a BigInt
    #[inline]
    pub fn is_bigint(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_BIGINT
    }

    /// Check if this is null or undefined (nullish)
    #[inline]
    pub fn is_nullish(&self) -> bool {
        self.0 == TAG_NULL || self.0 == TAG_UNDEFINED
    }

    /// Check if this is a pointer to a heap object
    #[inline]
    pub fn is_heap_object(&self) -> bool {
        // Check if it's any of the heap object types (string, object, array, function, bigint)
        let tag = self.0 & TAG_MASK;
        tag == TAG_POINTER || tag == TAG_BIGINT
    }

    /// Check if this is a primitive value (not a heap object)
    #[inline]
    pub fn is_primitive(&self) -> bool {
        !self.is_heap_object() || self.is_string()
    }

    // ==================== Value Extraction ====================

    /// Get as f64 if this is a number
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        if self.is_number() {
            Some(f64::from_bits(self.0))
        } else if self.is_integer() {
            // Convert inline integer to f64
            // Safety: is_integer() guarantees as_i32() returns Some
            self.as_i32().map(|i| i as f64)
        } else {
            None
        }
    }

    /// Get as i32 if this is an inline integer
    #[inline]
    pub fn as_i32(&self) -> Option<i32> {
        if self.is_integer() {
            Some((self.0 & INT_PAYLOAD_MASK) as u32 as i32)
        } else {
            None
        }
    }

    /// Get as bool if this is a boolean
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if self.is_boolean() {
            Some((self.0 & 1) != 0)
        } else {
            None
        }
    }

    /// Get as symbol ID if this is a symbol
    #[inline]
    pub fn as_symbol(&self) -> Option<u32> {
        if self.is_symbol() {
            Some((self.0 & INT_PAYLOAD_MASK) as u32)
        } else {
            None
        }
    }

    /// Get raw pointer if this is a heap object
    #[inline]
    pub fn as_ptr(&self) -> Option<*const u8> {
        if self.is_heap_object() {
            // For objects, arrays, and functions, the subtype bits occupy bits 32-47,
            // so we only extract bits 0-31 as the pointer
            if self.is_object() || self.is_array() || self.is_function() {
                Some((self.0 & 0x0000_0000_FFFF_FFFF) as *const u8)
            } else {
                // For strings and other heap objects, use full 48-bit payload
                Some((self.0 & PAYLOAD_MASK) as *const u8)
            }
        } else {
            None
        }
    }

    /// Get raw pointer as NonNull if this is a heap object
    #[inline]
    pub fn as_non_null(&self) -> Option<NonNull<u8>> {
        self.as_ptr().and_then(|p| NonNull::new(p as *mut u8))
    }

    // ==================== Type Coercion (ECMAScript semantics) ====================

    /// Convert to boolean (ToBoolean)
    ///
    /// Follows ECMAScript specification:
    /// - undefined, null -> false
    /// - boolean -> identity
    /// - number -> false if +0, -0, or NaN; true otherwise
    /// - string -> false if empty; true otherwise
    /// - symbol, bigint, object -> true
    #[inline]
    pub fn to_boolean(&self) -> bool {
        if self.is_undefined() || self.is_null() {
            return false;
        }
        if let Some(b) = self.as_bool() {
            return b;
        }
        if let Some(n) = self.as_f64() {
            return n != 0.0 && !n.is_nan();
        }
        if self.is_integer() {
            // Safety: is_integer() guarantees as_i32() returns Some
            return self.as_i32().is_some_and(|i| i != 0);
        }
        // Strings: would need to check length, but we don't have access here
        // For now, assume non-null pointer means truthy
        // Objects, symbols, bigints are always truthy
        true
    }

    /// Check if value is truthy (alias for to_boolean)
    #[inline]
    pub fn is_truthy(&self) -> bool {
        self.to_boolean()
    }

    /// Check if value is falsy
    #[inline]
    pub fn is_falsy(&self) -> bool {
        !self.to_boolean()
    }

    /// Convert to number (ToNumber)
    ///
    /// Follows ECMAScript specification:
    /// - undefined -> NaN
    /// - null -> +0
    /// - boolean -> 1 if true, +0 if false
    /// - number -> identity
    /// - string -> parse as number
    /// - symbol -> TypeError (returns NaN here)
    /// - bigint -> TypeError (returns NaN here)
    /// - object -> ToPrimitive then ToNumber
    #[inline]
    pub fn to_number(&self) -> f64 {
        if self.is_undefined() {
            return f64::NAN;
        }
        if self.is_null() {
            return 0.0;
        }
        if let Some(b) = self.as_bool() {
            return if b { 1.0 } else { 0.0 };
        }
        if let Some(n) = self.as_f64() {
            return n;
        }
        if let Some(i) = self.as_i32() {
            return i as f64;
        }
        if self.is_symbol() || self.is_bigint() {
            return f64::NAN; // Should throw TypeError
        }
        // For strings and objects, would need runtime context
        f64::NAN
    }

    /// Get the JavaScript typeof result
    #[inline]
    pub fn type_of(&self) -> &'static str {
        if self.is_undefined() {
            "undefined"
        } else if self.is_null() {
            "object" // typeof null === "object" (historical bug in JS)
        } else if self.is_boolean() {
            "boolean"
        } else if self.is_number() || self.is_integer() {
            "number"
        } else if self.is_string() {
            "string"
        } else if self.is_symbol() {
            "symbol"
        } else if self.is_bigint() {
            "bigint"
        } else if self.is_function() {
            "function"
        } else {
            "object"
        }
    }

    // ==================== Comparison ====================

    /// Strict equality (===)
    #[inline]
    pub fn strict_equals(&self, other: &TaggedValue) -> bool {
        // Same bits means same value for most cases
        if self.0 == other.0 {
            // Special case: NaN !== NaN
            if self.is_number() {
                let n = f64::from_bits(self.0);
                return !n.is_nan();
            }
            return true;
        }

        // Different types are never strictly equal
        if self.type_of() != other.type_of() {
            return false;
        }

        // For numbers, compare values (handles -0 === +0)
        if self.is_number() && other.is_number() {
            let a = f64::from_bits(self.0);
            let b = f64::from_bits(other.0);
            return a == b;
        }

        // Integer vs float comparison
        if (self.is_integer() && other.is_number()) || (self.is_number() && other.is_integer()) {
            return self.to_number() == other.to_number();
        }

        false
    }

    /// Same value (Object.is semantics)
    #[inline]
    pub fn same_value(&self, other: &TaggedValue) -> bool {
        // Same bits means same value
        if self.0 == other.0 {
            return true;
        }

        // For numbers, need special handling
        if self.is_number() && other.is_number() {
            let a = f64::from_bits(self.0);
            let b = f64::from_bits(other.0);

            // NaN is same value as NaN
            if a.is_nan() && b.is_nan() {
                return true;
            }

            // +0 is not same value as -0
            if a == 0.0 && b == 0.0 {
                return a.to_bits() == b.to_bits();
            }

            return a == b;
        }

        false
    }

    // ==================== Raw Access ====================

    /// Get raw bits (for debugging/serialization)
    #[inline]
    pub const fn bits(&self) -> u64 {
        self.0
    }

    /// Create from raw bits (unsafe - caller must ensure valid encoding)
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}

impl Default for TaggedValue {
    fn default() -> Self {
        Self::undefined()
    }
}

impl PartialEq for TaggedValue {
    fn eq(&self, other: &Self) -> bool {
        self.strict_equals(other)
    }
}

impl fmt::Debug for TaggedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_undefined() {
            write!(f, "undefined")
        } else if self.is_null() {
            write!(f, "null")
        } else if let Some(b) = self.as_bool() {
            write!(f, "{}", b)
        } else if let Some(i) = self.as_i32() {
            write!(f, "{}", i)
        } else if let Some(n) = self.as_f64() {
            write!(f, "{}", n)
        } else if let Some(sym) = self.as_symbol() {
            write!(f, "Symbol({})", sym)
        } else if let Some(ptr) = self.as_ptr() {
            if self.is_string() {
                write!(f, "String@{:p}", ptr)
            } else if self.is_object() {
                write!(f, "Object@{:p}", ptr)
            } else if self.is_array() {
                write!(f, "Array@{:p}", ptr)
            } else if self.is_function() {
                write!(f, "Function@{:p}", ptr)
            } else if self.is_bigint() {
                write!(f, "BigInt@{:p}", ptr)
            } else {
                write!(f, "HeapObject@{:p}", ptr)
            }
        } else {
            write!(f, "TaggedValue(0x{:016x})", self.0)
        }
    }
}

impl fmt::Display for TaggedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_undefined() {
            write!(f, "undefined")
        } else if self.is_null() {
            write!(f, "null")
        } else if let Some(b) = self.as_bool() {
            write!(f, "{}", b)
        } else if let Some(i) = self.as_i32() {
            write!(f, "{}", i)
        } else if let Some(n) = self.as_f64() {
            // Format numbers like JavaScript
            if n.is_nan() {
                write!(f, "NaN")
            } else if n.is_infinite() {
                if n.is_sign_positive() {
                    write!(f, "Infinity")
                } else {
                    write!(f, "-Infinity")
                }
            } else if n == 0.0 {
                write!(f, "0")
            } else if n.fract() == 0.0 && n.abs() < 1e15 {
                write!(f, "{}", n as i64)
            } else {
                write!(f, "{}", n)
            }
        } else if self.is_symbol() {
            write!(f, "Symbol()")
        } else if self.is_string() {
            write!(f, "[string]")
        } else if self.is_object() {
            write!(f, "[object Object]")
        } else if self.is_array() {
            write!(f, "[array]")
        } else if self.is_function() {
            write!(f, "[Function]")
        } else if self.is_bigint() {
            write!(f, "[bigint]")
        } else {
            write!(f, "[unknown]")
        }
    }
}

// ==================== Conversion From Standard Types ====================

impl From<f64> for TaggedValue {
    fn from(n: f64) -> Self {
        Self::from_f64(n)
    }
}

impl From<i32> for TaggedValue {
    fn from(n: i32) -> Self {
        Self::from_i32(n)
    }
}

impl From<bool> for TaggedValue {
    fn from(b: bool) -> Self {
        Self::from_bool(b)
    }
}

impl From<()> for TaggedValue {
    fn from(_: ()) -> Self {
        Self::undefined()
    }
}

impl<T> From<Option<T>> for TaggedValue
where
    T: Into<TaggedValue>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => Self::null(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_values() {
        let v = TaggedValue::from_f64(1.234);
        assert!(v.is_number());
        assert!(!v.is_integer());
        assert_eq!(v.as_f64(), Some(1.234));
        assert_eq!(v.type_of(), "number");
    }

    #[test]
    fn test_i32_values() {
        let v = TaggedValue::from_i32(42);
        assert!(v.is_integer());
        assert_eq!(v.as_i32(), Some(42));
        assert_eq!(v.to_number(), 42.0);
        assert_eq!(v.type_of(), "number");

        // Test negative integers
        let neg = TaggedValue::from_i32(-100);
        assert_eq!(neg.as_i32(), Some(-100));
        assert_eq!(neg.to_number(), -100.0);
    }

    #[test]
    fn test_bool_values() {
        let t = TaggedValue::from_bool(true);
        let f = TaggedValue::from_bool(false);

        assert!(t.is_boolean());
        assert!(f.is_boolean());
        assert_eq!(t.as_bool(), Some(true));
        assert_eq!(f.as_bool(), Some(false));
        assert_eq!(t.type_of(), "boolean");

        assert!(t.is_truthy());
        assert!(!f.is_truthy());
    }

    #[test]
    fn test_null_undefined() {
        let null = TaggedValue::null();
        let undef = TaggedValue::undefined();

        assert!(null.is_null());
        assert!(!null.is_undefined());
        assert!(undef.is_undefined());
        assert!(!undef.is_null());

        assert!(null.is_nullish());
        assert!(undef.is_nullish());

        assert_eq!(null.type_of(), "object"); // typeof null === "object"
        assert_eq!(undef.type_of(), "undefined");

        assert!(!null.is_truthy());
        assert!(!undef.is_truthy());
    }

    #[test]
    fn test_type_coercion_to_boolean() {
        // Falsy values
        assert!(!TaggedValue::undefined().to_boolean());
        assert!(!TaggedValue::null().to_boolean());
        assert!(!TaggedValue::from_bool(false).to_boolean());
        assert!(!TaggedValue::from_f64(0.0).to_boolean());
        assert!(!TaggedValue::from_f64(-0.0).to_boolean());
        assert!(!TaggedValue::from_f64(f64::NAN).to_boolean());
        assert!(!TaggedValue::from_i32(0).to_boolean());

        // Truthy values
        assert!(TaggedValue::from_bool(true).to_boolean());
        assert!(TaggedValue::from_f64(1.0).to_boolean());
        assert!(TaggedValue::from_f64(-1.0).to_boolean());
        assert!(TaggedValue::from_i32(1).to_boolean());
        assert!(TaggedValue::from_i32(-1).to_boolean());
    }

    #[test]
    fn test_type_coercion_to_number() {
        assert!(TaggedValue::undefined().to_number().is_nan());
        assert_eq!(TaggedValue::null().to_number(), 0.0);
        assert_eq!(TaggedValue::from_bool(true).to_number(), 1.0);
        assert_eq!(TaggedValue::from_bool(false).to_number(), 0.0);
        assert_eq!(TaggedValue::from_f64(42.5).to_number(), 42.5);
        assert_eq!(TaggedValue::from_i32(42).to_number(), 42.0);
    }

    #[test]
    fn test_strict_equality() {
        // Same values
        assert!(TaggedValue::from_i32(42).strict_equals(&TaggedValue::from_i32(42)));
        assert!(TaggedValue::from_f64(1.234).strict_equals(&TaggedValue::from_f64(1.234)));
        assert!(TaggedValue::null().strict_equals(&TaggedValue::null()));
        assert!(TaggedValue::undefined().strict_equals(&TaggedValue::undefined()));

        // Different values
        assert!(!TaggedValue::from_i32(42).strict_equals(&TaggedValue::from_i32(43)));
        assert!(!TaggedValue::null().strict_equals(&TaggedValue::undefined()));

        // NaN !== NaN
        assert!(!TaggedValue::from_f64(f64::NAN).strict_equals(&TaggedValue::from_f64(f64::NAN)));

        // +0 === -0
        assert!(TaggedValue::from_f64(0.0).strict_equals(&TaggedValue::from_f64(-0.0)));
    }

    #[test]
    fn test_same_value() {
        // NaN is same value as NaN (Object.is semantics)
        assert!(TaggedValue::from_f64(f64::NAN).same_value(&TaggedValue::from_f64(f64::NAN)));

        // +0 is NOT same value as -0
        assert!(!TaggedValue::from_f64(0.0).same_value(&TaggedValue::from_f64(-0.0)));

        // Regular equality
        assert!(TaggedValue::from_i32(42).same_value(&TaggedValue::from_i32(42)));
    }

    #[test]
    fn test_symbol() {
        let sym = TaggedValue::from_symbol(123);
        assert!(sym.is_symbol());
        assert_eq!(sym.as_symbol(), Some(123));
        assert_eq!(sym.type_of(), "symbol");
    }

    #[test]
    fn test_display_numbers() {
        // Integer-like floats should display without decimal
        assert_eq!(format!("{}", TaggedValue::from_f64(42.0)), "42");
        assert_eq!(format!("{}", TaggedValue::from_i32(42)), "42");

        // Floats with decimals
        assert_eq!(format!("{}", TaggedValue::from_f64(1.234)), "1.234");

        // Special values
        assert_eq!(format!("{}", TaggedValue::from_f64(f64::NAN)), "NaN");
        assert_eq!(format!("{}", TaggedValue::from_f64(f64::INFINITY)), "Infinity");
        assert_eq!(format!("{}", TaggedValue::from_f64(f64::NEG_INFINITY)), "-Infinity");
    }

    #[test]
    fn test_from_conversions() {
        let v: TaggedValue = 42.0f64.into();
        assert!(v.is_number());

        let v: TaggedValue = 42i32.into();
        assert!(v.is_integer());

        let v: TaggedValue = true.into();
        assert!(v.is_boolean());

        let v: TaggedValue = ().into();
        assert!(v.is_undefined());

        let v: TaggedValue = None::<i32>.into();
        assert!(v.is_null());

        let v: TaggedValue = Some(42i32).into();
        assert!(v.is_integer());
    }
}
