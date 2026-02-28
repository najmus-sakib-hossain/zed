//! JavaScript value representation
//!
//! This module provides two value representations:
//!
//! 1. `TaggedValue` - A NaN-boxed 64-bit representation for efficient storage
//!    and fast type checking. Used in the JIT-compiled execution path.
//!
//! 2. `Value` - A Rust enum representation for easier manipulation in the
//!    interpreter and for interop with Rust code.

pub mod object;
pub mod string;
pub mod tagged;

use std::fmt;

// Re-export TaggedValue for use throughout the runtime
pub use tagged::{HeapObjectType, TaggedValue};

/// JavaScript value (enum representation)
///
/// This is the high-level value representation used for:
/// - Interpreter execution
/// - FFI boundaries
/// - Debugging and inspection
///
/// For performance-critical paths, use `TaggedValue` instead.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Undefined
    Undefined,
    /// Null
    Null,
    /// Boolean
    Boolean(bool),
    /// Number (f64)
    Number(f64),
    /// Integer (i32) - stored separately for optimization
    Integer(i32),
    /// String
    String(String),
    /// Symbol (with ID)
    Symbol(u32),
    /// BigInt (stored as string for now)
    BigInt(String),
    /// Object
    Object(object::Object),
    /// Array
    Array(Vec<Value>),
    /// Function
    Function(FunctionValue),
    /// Promise
    Promise(Box<PromiseValue>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionValue {
    pub name: String,
    pub ptr: usize,
}

/// Promise state
#[derive(Clone, Debug, PartialEq)]
pub enum PromiseState {
    Pending,
    Fulfilled(Box<Value>),
    Rejected(Box<Value>),
}

/// Promise value
#[derive(Clone, Debug, PartialEq)]
pub struct PromiseValue {
    pub state: PromiseState,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Undefined => write!(f, "undefined"),
            Value::Null => write!(f, "null"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Number(n) => {
                // Format numbers like JavaScript
                if n.is_nan() {
                    write!(f, "NaN")
                } else if n.is_infinite() {
                    if n.is_sign_positive() {
                        write!(f, "Infinity")
                    } else {
                        write!(f, "-Infinity")
                    }
                } else if *n == 0.0 {
                    write!(f, "0")
                } else if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Integer(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{}", s),
            Value::Symbol(id) => write!(f, "Symbol({})", id),
            Value::BigInt(s) => write!(f, "{}n", s),
            Value::Object(_) => write!(f, "[object Object]"),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Function(func) => write!(f, "[Function: {}]", func.name),
            Value::Promise(p) => match &p.state {
                PromiseState::Pending => write!(f, "Promise {{ <pending> }}"),
                PromiseState::Fulfilled(v) => write!(f, "Promise {{ {} }}", v),
                PromiseState::Rejected(e) => write!(f, "Promise {{ <rejected> {} }}", e),
            },
        }
    }
}

impl Value {
    /// Check if value is truthy (ToBoolean)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Integer(i) => *i != 0,
            Value::String(s) => !s.is_empty(),
            Value::BigInt(s) => s != "0",
            _ => true,
        }
    }

    /// Convert to number (ToNumber)
    pub fn to_number(&self) -> f64 {
        match self {
            Value::Undefined => f64::NAN,
            Value::Null => 0.0,
            Value::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Number(n) => *n,
            Value::Integer(i) => *i as f64,
            Value::String(s) => s.trim().parse().unwrap_or(f64::NAN),
            Value::BigInt(_) => f64::NAN, // TypeError in real JS
            Value::Symbol(_) => f64::NAN, // TypeError in real JS
            _ => f64::NAN,
        }
    }

    /// Convert to i32 (ToInt32)
    pub fn to_i32(&self) -> i32 {
        let n = self.to_number();
        if n.is_nan() || n.is_infinite() || n == 0.0 {
            return 0;
        }
        let int_val = n.trunc();
        // Modulo 2^32 and convert to signed
        let uint32 = (int_val as i64).rem_euclid(0x1_0000_0000) as u32;
        uint32 as i32
    }

    /// Convert to u32 (ToUint32)
    pub fn to_u32(&self) -> u32 {
        let n = self.to_number();
        if n.is_nan() || n.is_infinite() || n == 0.0 {
            return 0;
        }
        let int_val = n.trunc();
        (int_val as i64).rem_euclid(0x1_0000_0000) as u32
    }

    /// Convert to string (ToString)
    pub fn to_js_string(&self) -> String {
        format!("{}", self)
    }

    /// Get the JavaScript type name (typeof)
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Undefined => "undefined",
            Value::Null => "object", // typeof null === "object" in JS
            Value::Boolean(_) => "boolean",
            Value::Number(_) | Value::Integer(_) => "number",
            Value::String(_) => "string",
            Value::Symbol(_) => "symbol",
            Value::BigInt(_) => "bigint",
            Value::Object(_) => "object",
            Value::Array(_) => "object", // typeof [] === "object" in JS
            Value::Function(_) => "function",
            Value::Promise(_) => "object",
        }
    }

    /// Check if value is null or undefined
    pub fn is_nullish(&self) -> bool {
        matches!(self, Value::Null | Value::Undefined)
    }

    /// Check if value is an object (including arrays)
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_) | Value::Array(_))
    }

    /// Check if value is a function
    pub fn is_function(&self) -> bool {
        matches!(self, Value::Function(_))
    }

    /// Check if value is a number
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_) | Value::Integer(_))
    }

    /// Check if value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if value is a boolean
    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    /// Check if value is a symbol
    pub fn is_symbol(&self) -> bool {
        matches!(self, Value::Symbol(_))
    }

    /// Check if value is a BigInt
    pub fn is_bigint(&self) -> bool {
        matches!(self, Value::BigInt(_))
    }

    /// Strict equality (===)
    pub fn strict_equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) => true,
            (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => {
                // NaN !== NaN, but +0 === -0
                if a.is_nan() || b.is_nan() {
                    false
                } else {
                    a == b
                }
            }
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Number(a), Value::Integer(b)) => *a == (*b as f64),
            (Value::Integer(a), Value::Number(b)) => (*a as f64) == *b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::BigInt(a), Value::BigInt(b)) => a == b,
            // Objects compare by reference (identity)
            _ => false,
        }
    }

    /// Loose equality (==)
    pub fn loose_equals(&self, other: &Value) -> bool {
        // Same type: use strict equality
        if std::mem::discriminant(self) == std::mem::discriminant(other) {
            return self.strict_equals(other);
        }

        // null == undefined
        if self.is_nullish() && other.is_nullish() {
            return true;
        }

        // Number comparisons with type coercion
        match (self, other) {
            (Value::Number(_) | Value::Integer(_), Value::String(_)) => {
                self.to_number() == other.to_number()
            }
            (Value::String(_), Value::Number(_) | Value::Integer(_)) => {
                self.to_number() == other.to_number()
            }
            (Value::Boolean(_), _) => Value::Number(self.to_number()).loose_equals(other),
            (_, Value::Boolean(_)) => self.loose_equals(&Value::Number(other.to_number())),
            _ => false,
        }
    }

    /// Convert to TaggedValue for efficient storage
    pub fn to_tagged(&self) -> TaggedValue {
        match self {
            Value::Undefined => TaggedValue::undefined(),
            Value::Null => TaggedValue::null(),
            Value::Boolean(b) => TaggedValue::from_bool(*b),
            Value::Number(n) => TaggedValue::from_f64(*n),
            Value::Integer(i) => TaggedValue::from_i32(*i),
            Value::Symbol(id) => TaggedValue::from_symbol(*id),
            // For heap-allocated types, we can't convert without a GC context
            // Return undefined as a fallback (caller should handle these cases)
            _ => TaggedValue::undefined(),
        }
    }

    /// Create from TaggedValue (primitives only)
    pub fn from_tagged(tagged: TaggedValue) -> Self {
        if tagged.is_undefined() {
            Value::Undefined
        } else if tagged.is_null() {
            Value::Null
        } else if let Some(b) = tagged.as_bool() {
            Value::Boolean(b)
        } else if let Some(i) = tagged.as_i32() {
            Value::Integer(i)
        } else if let Some(n) = tagged.as_f64() {
            Value::Number(n)
        } else if let Some(id) = tagged.as_symbol() {
            Value::Symbol(id)
        } else {
            // Heap objects need special handling with GC context
            Value::Undefined
        }
    }
}
