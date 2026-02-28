//! PyTuple - Python tuple type

use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use crate::pylist::PyValue;
use crate::{RuntimeError, RuntimeResult};
use std::sync::Arc;

/// Python tuple object (immutable)
pub struct PyTuple {
    /// Object header
    pub header: PyObjectHeader,
    /// Tuple elements (immutable after creation)
    elements: Arc<[PyValue]>,
    /// Cached hash
    hash: Option<u64>,
}

impl PyTuple {
    /// Create a new tuple from values
    pub fn from_values(values: Vec<PyValue>) -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::Tuple, ObjectFlags::IMMUTABLE),
            elements: values.into(),
            hash: None,
        }
    }

    /// Create an empty tuple
    pub fn empty() -> Self {
        Self::from_values(Vec::new())
    }

    /// Create a tuple with a single element
    pub fn single(value: PyValue) -> Self {
        Self::from_values(vec![value])
    }

    /// Create a tuple with two elements
    pub fn pair(a: PyValue, b: PyValue) -> Self {
        Self::from_values(vec![a, b])
    }

    /// Get length
    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get item at index
    pub fn getitem(&self, index: i64) -> RuntimeResult<PyValue> {
        let len = self.elements.len() as i64;
        let idx = if index < 0 { len + index } else { index };

        if idx < 0 || idx >= len {
            return Err(RuntimeError::index_error(index, self.elements.len()));
        }

        Ok(self.elements[idx as usize].clone())
    }

    /// Get slice
    pub fn slice(&self, start: Option<i64>, end: Option<i64>) -> PyTuple {
        let len = self.elements.len() as i64;

        let start = match start {
            Some(s) if s < 0 => (len + s).max(0) as usize,
            Some(s) => (s as usize).min(self.elements.len()),
            None => 0,
        };

        let end = match end {
            Some(e) if e < 0 => (len + e).max(0) as usize,
            Some(e) => (e as usize).min(self.elements.len()),
            None => self.elements.len(),
        };

        if start >= end {
            return PyTuple::empty();
        }

        PyTuple::from_values(self.elements[start..end].to_vec())
    }

    /// Check if contains value
    pub fn contains(&self, value: &PyValue) -> bool {
        self.elements.iter().any(|v| Self::values_equal(v, value))
    }

    /// Count occurrences
    pub fn count(&self, value: &PyValue) -> usize {
        self.elements.iter().filter(|v| Self::values_equal(v, value)).count()
    }

    /// Find index of value
    pub fn index(&self, value: &PyValue) -> RuntimeResult<usize> {
        self.elements
            .iter()
            .position(|v| Self::values_equal(v, value))
            .ok_or_else(|| RuntimeError::value_error("value not in tuple"))
    }

    /// Concatenate two tuples
    pub fn concat(&self, other: &PyTuple) -> PyTuple {
        let mut elements = self.elements.to_vec();
        elements.extend(other.elements.iter().cloned());
        PyTuple::from_values(elements)
    }

    /// Repeat tuple n times
    pub fn repeat(&self, n: usize) -> PyTuple {
        let mut result = Vec::with_capacity(self.elements.len() * n);
        for _ in 0..n {
            result.extend(self.elements.iter().cloned());
        }
        PyTuple::from_values(result)
    }

    /// Get all elements as a vector
    pub fn to_vec(&self) -> Vec<PyValue> {
        self.elements.to_vec()
    }

    /// Get elements as slice
    pub fn as_slice(&self) -> &[PyValue] {
        &self.elements
    }

    /// Compute hash (for use as dict key)
    pub fn hash(&self) -> RuntimeResult<u64> {
        let mut hash: u64 = 0x345678;
        let mult: u64 = 1000003;
        let len = self.elements.len() as u64;

        for elem in self.elements.iter() {
            let elem_hash = Self::hash_value(elem)?;
            hash = (hash ^ elem_hash).wrapping_mul(mult);
        }

        Ok(hash.wrapping_add(len))
    }

    /// Hash a single value
    fn hash_value(value: &PyValue) -> RuntimeResult<u64> {
        match value {
            PyValue::None => Ok(0),
            PyValue::Bool(b) => Ok(*b as u64),
            PyValue::Int(i) => Ok(*i as u64),
            PyValue::Str(s) => {
                let mut hash: u64 = 0xcbf29ce484222325;
                for byte in s.bytes() {
                    hash ^= byte as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                Ok(hash)
            }
            PyValue::Tuple(t) => t.hash(),
            _ => Err(RuntimeError::type_error("hashable", value.type_name())),
        }
    }

    /// Simple value equality check
    fn values_equal(a: &PyValue, b: &PyValue) -> bool {
        match (a, b) {
            (PyValue::None, PyValue::None) => true,
            (PyValue::Bool(x), PyValue::Bool(y)) => x == y,
            (PyValue::Int(x), PyValue::Int(y)) => x == y,
            (PyValue::Float(x), PyValue::Float(y)) => x == y,
            (PyValue::Str(x), PyValue::Str(y)) => x == y,
            _ => false,
        }
    }
}

impl std::fmt::Debug for PyTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PyTuple(")?;
        if self.elements.len() == 1 {
            write!(f, "{:?},", self.elements[0])?;
        } else {
            for (i, elem) in self.elements.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{:?}", elem)?;
            }
        }
        write!(f, ")")
    }
}

impl Clone for PyTuple {
    fn clone(&self) -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::Tuple, ObjectFlags::IMMUTABLE),
            elements: Arc::clone(&self.elements),
            hash: self.hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_creation() {
        let tuple = PyTuple::from_values(vec![PyValue::Int(1), PyValue::Int(2), PyValue::Int(3)]);
        assert_eq!(tuple.len(), 3);
        assert_eq!(tuple.header.type_tag(), TypeTag::Tuple);
    }

    #[test]
    fn test_tuple_getitem() {
        let tuple = PyTuple::from_values(vec![PyValue::Int(1), PyValue::Int(2), PyValue::Int(3)]);

        if let PyValue::Int(v) = tuple.getitem(0).unwrap() {
            assert_eq!(v, 1);
        }

        if let PyValue::Int(v) = tuple.getitem(-1).unwrap() {
            assert_eq!(v, 3);
        }
    }

    #[test]
    fn test_tuple_slice() {
        let tuple = PyTuple::from_values(vec![
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Int(3),
            PyValue::Int(4),
        ]);

        let slice = tuple.slice(Some(1), Some(3));
        assert_eq!(slice.len(), 2);
    }

    #[test]
    fn test_tuple_concat() {
        let a = PyTuple::from_values(vec![PyValue::Int(1)]);
        let b = PyTuple::from_values(vec![PyValue::Int(2)]);
        let c = a.concat(&b);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn test_tuple_hash() {
        let tuple = PyTuple::from_values(vec![PyValue::Int(1), PyValue::Int(2)]);

        let hash1 = tuple.hash().unwrap();
        let hash2 = tuple.hash().unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_empty_tuple() {
        let tuple = PyTuple::empty();
        assert!(tuple.is_empty());
    }
}
