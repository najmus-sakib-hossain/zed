//! PyInt - Python integer type

use crate::error::{RuntimeError, RuntimeResult};
use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};

/// Python integer object
///
/// Uses i64 for small integers, with potential bigint support later
pub struct PyInt {
    /// Object header
    pub header: PyObjectHeader,
    /// Integer value (small int optimization)
    value: i64,
}

impl PyInt {
    /// Create a new integer
    pub fn new(value: i64) -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::Int, ObjectFlags::IMMUTABLE),
            value,
        }
    }

    /// Get the value
    #[inline]
    pub fn value(&self) -> i64 {
        self.value
    }

    /// Add two integers
    pub fn add(&self, other: &PyInt) -> RuntimeResult<PyInt> {
        self.value
            .checked_add(other.value)
            .map(PyInt::new)
            .ok_or(RuntimeError::overflow_error("addition"))
    }

    /// Subtract two integers
    pub fn sub(&self, other: &PyInt) -> RuntimeResult<PyInt> {
        self.value
            .checked_sub(other.value)
            .map(PyInt::new)
            .ok_or(RuntimeError::overflow_error("subtraction"))
    }

    /// Multiply two integers
    pub fn mul(&self, other: &PyInt) -> RuntimeResult<PyInt> {
        self.value
            .checked_mul(other.value)
            .map(PyInt::new)
            .ok_or(RuntimeError::overflow_error("multiplication"))
    }

    /// Floor divide two integers
    pub fn floordiv(&self, other: &PyInt) -> RuntimeResult<PyInt> {
        if other.value == 0 {
            return Err(RuntimeError::ZeroDivisionError);
        }
        Ok(PyInt::new(self.value / other.value))
    }

    /// Modulo two integers
    pub fn modulo(&self, other: &PyInt) -> RuntimeResult<PyInt> {
        if other.value == 0 {
            return Err(RuntimeError::ZeroDivisionError);
        }
        Ok(PyInt::new(self.value % other.value))
    }

    /// Power (with overflow check)
    pub fn pow(&self, exp: &PyInt) -> RuntimeResult<PyInt> {
        if exp.value < 0 {
            return Err(RuntimeError::value_error("negative exponent"));
        }
        let mut result: i64 = 1;
        let mut base = self.value;
        let mut exp = exp.value as u64;

        while exp > 0 {
            if exp & 1 == 1 {
                result = result.checked_mul(base).ok_or(RuntimeError::overflow_error("power"))?;
            }
            exp >>= 1;
            if exp > 0 {
                base = base.checked_mul(base).ok_or(RuntimeError::overflow_error("power"))?;
            }
        }
        Ok(PyInt::new(result))
    }

    /// Negate
    pub fn neg(&self) -> RuntimeResult<PyInt> {
        self.value
            .checked_neg()
            .map(PyInt::new)
            .ok_or(RuntimeError::overflow_error("negation"))
    }

    /// Absolute value
    pub fn abs(&self) -> RuntimeResult<PyInt> {
        self.value
            .checked_abs()
            .map(PyInt::new)
            .ok_or(RuntimeError::overflow_error("absolute value"))
    }

    /// Bitwise AND
    pub fn bitand(&self, other: &PyInt) -> PyInt {
        PyInt::new(self.value & other.value)
    }

    /// Bitwise OR
    pub fn bitor(&self, other: &PyInt) -> PyInt {
        PyInt::new(self.value | other.value)
    }

    /// Bitwise XOR
    pub fn bitxor(&self, other: &PyInt) -> PyInt {
        PyInt::new(self.value ^ other.value)
    }

    /// Bitwise NOT
    pub fn invert(&self) -> PyInt {
        PyInt::new(!self.value)
    }

    /// Left shift
    pub fn lshift(&self, other: &PyInt) -> RuntimeResult<PyInt> {
        if other.value < 0 {
            return Err(RuntimeError::value_error("negative shift count"));
        }
        if other.value >= 64 {
            return Err(RuntimeError::overflow_error("left shift"));
        }
        Ok(PyInt::new(self.value << other.value))
    }

    /// Right shift
    pub fn rshift(&self, other: &PyInt) -> RuntimeResult<PyInt> {
        if other.value < 0 {
            return Err(RuntimeError::value_error("negative shift count"));
        }
        if other.value >= 64 {
            return Ok(PyInt::new(if self.value < 0 { -1 } else { 0 }));
        }
        Ok(PyInt::new(self.value >> other.value))
    }

    /// Compare
    pub fn cmp(&self, other: &PyInt) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }

    /// Hash
    pub fn hash(&self) -> u64 {
        // Simple hash for integers
        self.value as u64
    }

    /// Convert to bool
    pub fn to_bool(&self) -> bool {
        self.value != 0
    }

    /// Convert to float
    pub fn to_float(&self) -> f64 {
        self.value as f64
    }
}

impl std::fmt::Display for PyInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::fmt::Debug for PyInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PyInt({})", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_creation() {
        let i = PyInt::new(42);
        assert_eq!(i.value(), 42);
        assert_eq!(i.header.type_tag(), TypeTag::Int);
    }

    #[test]
    fn test_int_arithmetic() {
        let a = PyInt::new(10);
        let b = PyInt::new(3);

        assert_eq!(a.add(&b).unwrap().value(), 13);
        assert_eq!(a.sub(&b).unwrap().value(), 7);
        assert_eq!(a.mul(&b).unwrap().value(), 30);
        assert_eq!(a.floordiv(&b).unwrap().value(), 3);
        assert_eq!(a.modulo(&b).unwrap().value(), 1);
    }

    #[test]
    fn test_int_bitwise() {
        let a = PyInt::new(0b1010);
        let b = PyInt::new(0b1100);

        assert_eq!(a.bitand(&b).value(), 0b1000);
        assert_eq!(a.bitor(&b).value(), 0b1110);
        assert_eq!(a.bitxor(&b).value(), 0b0110);
    }

    #[test]
    fn test_int_overflow() {
        let max = PyInt::new(i64::MAX);
        let one = PyInt::new(1);

        assert!(max.add(&one).is_err());
    }

    #[test]
    fn test_division_by_zero() {
        let a = PyInt::new(10);
        let zero = PyInt::new(0);

        assert!(a.floordiv(&zero).is_err());
        assert!(a.modulo(&zero).is_err());
    }
}
