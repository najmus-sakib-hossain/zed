//! PyDict - Python dictionary type

use crate::error::{RuntimeError, RuntimeResult};
use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use crate::pylist::PyValue;
use dashmap::DashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Hashable key for dictionary
#[derive(Clone, Eq)]
pub enum PyKey {
    None,
    Bool(bool),
    Int(i64),
    Str(Arc<str>),
    Tuple(Vec<PyKey>),
}

impl PartialEq for PyKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PyKey::None, PyKey::None) => true,
            (PyKey::Bool(a), PyKey::Bool(b)) => a == b,
            (PyKey::Int(a), PyKey::Int(b)) => a == b,
            (PyKey::Str(a), PyKey::Str(b)) => a == b,
            (PyKey::Tuple(a), PyKey::Tuple(b)) => a == b,
            // Cross-type comparisons for bool/int
            (PyKey::Bool(b), PyKey::Int(i)) | (PyKey::Int(i), PyKey::Bool(b)) => *i == (*b as i64),
            _ => false,
        }
    }
}

impl Hash for PyKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PyKey::None => 0u8.hash(state),
            PyKey::Bool(b) => (*b as i64).hash(state),
            PyKey::Int(i) => i.hash(state),
            PyKey::Str(s) => s.hash(state),
            PyKey::Tuple(t) => {
                for item in t {
                    item.hash(state);
                }
            }
        }
    }
}

impl std::fmt::Debug for PyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PyKey::None => write!(f, "None"),
            PyKey::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            PyKey::Int(i) => write!(f, "{}", i),
            PyKey::Str(s) => write!(f, "'{}'", s),
            PyKey::Tuple(t) => write!(f, "{:?}", t),
        }
    }
}

impl PyKey {
    /// Try to convert a PyValue to a PyKey
    pub fn from_value(value: &PyValue) -> RuntimeResult<Self> {
        match value {
            PyValue::None => Ok(PyKey::None),
            PyValue::Bool(b) => Ok(PyKey::Bool(*b)),
            PyValue::Int(i) => Ok(PyKey::Int(*i)),
            PyValue::Str(s) => Ok(PyKey::Str(Arc::clone(s))),
            PyValue::Tuple(t) => {
                let keys: RuntimeResult<Vec<PyKey>> =
                    t.to_vec().iter().map(PyKey::from_value).collect();
                Ok(PyKey::Tuple(keys?))
            }
            _ => Err(RuntimeError::type_error("hashable type", value.type_name())),
        }
    }

    /// Convert key to value
    pub fn to_value(&self) -> PyValue {
        match self {
            PyKey::None => PyValue::None,
            PyKey::Bool(b) => PyValue::Bool(*b),
            PyKey::Int(i) => PyValue::Int(*i),
            PyKey::Str(s) => PyValue::Str(Arc::clone(s)),
            PyKey::Tuple(t) => {
                let values: Vec<PyValue> = t.iter().map(|k| k.to_value()).collect();
                PyValue::Tuple(Arc::new(crate::PyTuple::from_values(values)))
            }
        }
    }
}

/// Python dictionary object
pub struct PyDict {
    /// Object header
    pub header: PyObjectHeader,
    /// Dictionary entries (thread-safe)
    entries: DashMap<PyKey, PyValue>,
}

impl PyDict {
    /// Create a new empty dictionary
    pub fn new() -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::Dict, ObjectFlags::NONE),
            entries: DashMap::new(),
        }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::Dict, ObjectFlags::NONE),
            entries: DashMap::with_capacity(capacity),
        }
    }

    /// Get length
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get item by key
    pub fn getitem(&self, key: &PyKey) -> RuntimeResult<PyValue> {
        self.entries
            .get(key)
            .map(|v| v.clone())
            .ok_or_else(|| RuntimeError::key_error(format!("{:?}", key)))
    }

    /// Get item with default
    pub fn get(&self, key: &PyKey, default: PyValue) -> PyValue {
        self.entries.get(key).map(|v| v.clone()).unwrap_or(default)
    }

    /// Set item
    pub fn setitem(&self, key: PyKey, value: PyValue) {
        self.entries.insert(key, value);
    }

    /// Delete item
    pub fn delitem(&self, key: &PyKey) -> RuntimeResult<()> {
        self.entries
            .remove(key)
            .map(|_| ())
            .ok_or_else(|| RuntimeError::key_error(format!("{:?}", key)))
    }

    /// Check if contains key
    pub fn contains(&self, key: &PyKey) -> bool {
        self.entries.contains_key(key)
    }

    /// Clear the dictionary
    pub fn clear(&self) {
        self.entries.clear();
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<PyKey> {
        self.entries.iter().map(|r| r.key().clone()).collect()
    }

    /// Get all values
    pub fn values(&self) -> Vec<PyValue> {
        self.entries.iter().map(|r| r.value().clone()).collect()
    }

    /// Get all items as (key, value) pairs
    pub fn items(&self) -> Vec<(PyKey, PyValue)> {
        self.entries.iter().map(|r| (r.key().clone(), r.value().clone())).collect()
    }

    /// Pop item with key
    pub fn pop(&self, key: &PyKey, default: Option<PyValue>) -> RuntimeResult<PyValue> {
        match self.entries.remove(key) {
            Some((_, v)) => Ok(v),
            None => default.ok_or_else(|| RuntimeError::key_error(format!("{:?}", key))),
        }
    }

    /// Pop arbitrary item
    pub fn popitem(&self) -> RuntimeResult<(PyKey, PyValue)> {
        // Get first key
        let key = self.entries.iter().next().map(|r| r.key().clone());

        match key {
            Some(k) => {
                let v = self.entries.remove(&k).unwrap().1;
                Ok((k, v))
            }
            None => Err(RuntimeError::key_error("dictionary is empty")),
        }
    }

    /// Set default value if key doesn't exist
    pub fn setdefault(&self, key: PyKey, default: PyValue) -> PyValue {
        self.entries.entry(key).or_insert(default).clone()
    }

    /// Update with items from another dict
    pub fn update(&self, other: &PyDict) {
        for item in other.entries.iter() {
            self.entries.insert(item.key().clone(), item.value().clone());
        }
    }

    /// Create a shallow copy
    pub fn copy(&self) -> PyDict {
        let new_dict = PyDict::new();
        for item in self.entries.iter() {
            new_dict.entries.insert(item.key().clone(), item.value().clone());
        }
        new_dict
    }

    // ============================================
    // Python-facing methods that return PyValue
    // ============================================

    /// dict.keys() -> list of keys as PyValue
    /// Returns a PyValue::List containing all keys
    pub fn keys_list(&self) -> PyValue {
        let keys: Vec<PyValue> = self.entries.iter().map(|r| r.key().to_value()).collect();
        PyValue::List(std::sync::Arc::new(crate::pylist::PyList::from_values(keys)))
    }

    /// dict.values() -> list of values as PyValue
    /// Returns a PyValue::List containing all values
    pub fn values_list(&self) -> PyValue {
        let values: Vec<PyValue> = self.entries.iter().map(|r| r.value().clone()).collect();
        PyValue::List(std::sync::Arc::new(crate::pylist::PyList::from_values(values)))
    }

    /// dict.items() -> list of (key, value) tuples as PyValue
    /// Returns a PyValue::List containing PyValue::Tuple pairs
    pub fn items_list(&self) -> PyValue {
        let items: Vec<PyValue> = self
            .entries
            .iter()
            .map(|r| {
                let key = r.key().to_value();
                let value = r.value().clone();
                PyValue::Tuple(std::sync::Arc::new(crate::PyTuple::pair(key, value)))
            })
            .collect();
        PyValue::List(std::sync::Arc::new(crate::pylist::PyList::from_values(items)))
    }

    /// dict.get(key, default=None) -> value
    /// Returns the value for key if present, otherwise returns default
    pub fn get_with_default(&self, key: &PyKey, default: PyValue) -> PyValue {
        self.entries.get(key).map(|v| v.clone()).unwrap_or(default)
    }

    /// dict.pop(key, default?) -> value
    /// Removes and returns the value for key, or default if key not found
    /// Raises KeyError if key not found and no default provided
    pub fn pop_with_default(&self, key: &PyKey, default: Option<PyValue>) -> RuntimeResult<PyValue> {
        match self.entries.remove(key) {
            Some((_, v)) => Ok(v),
            None => default.ok_or_else(|| RuntimeError::key_error(format!("{:?}", key))),
        }
    }

    /// dict.update(other) - merges other dict into self
    /// Updates the dictionary with key-value pairs from other
    pub fn update_from(&self, other: &PyDict) {
        for item in other.entries.iter() {
            self.entries.insert(item.key().clone(), item.value().clone());
        }
    }
}

impl Default for PyDict {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PyDict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PyDict({{")?;
        let items: Vec<_> = self.entries.iter().collect();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}: {:?}", item.key(), item.value())?;
        }
        write!(f, "}})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_creation() {
        let dict = PyDict::new();
        assert!(dict.is_empty());
        assert_eq!(dict.header.type_tag(), TypeTag::Dict);
    }

    #[test]
    fn test_dict_set_get() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Str(Arc::from("key")), PyValue::Int(42));

        let value = dict.getitem(&PyKey::Str(Arc::from("key"))).unwrap();
        if let PyValue::Int(v) = value {
            assert_eq!(v, 42);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_dict_contains() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("one")));

        assert!(dict.contains(&PyKey::Int(1)));
        assert!(!dict.contains(&PyKey::Int(2)));
    }

    #[test]
    fn test_dict_delete() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Int(100));

        assert!(dict.delitem(&PyKey::Int(1)).is_ok());
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dict_keys_values() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("a")));
        dict.setitem(PyKey::Int(2), PyValue::Str(Arc::from("b")));

        assert_eq!(dict.keys().len(), 2);
        assert_eq!(dict.values().len(), 2);
    }

    #[test]
    fn test_dict_update() {
        let dict1 = PyDict::new();
        dict1.setitem(PyKey::Int(1), PyValue::Int(1));

        let dict2 = PyDict::new();
        dict2.setitem(PyKey::Int(2), PyValue::Int(2));

        dict1.update(&dict2);
        assert_eq!(dict1.len(), 2);
    }

    #[test]
    fn test_dict_keys_list() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("a")));
        dict.setitem(PyKey::Int(2), PyValue::Str(Arc::from("b")));

        let keys = dict.keys_list();
        if let PyValue::List(list) = keys {
            assert_eq!(list.len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_dict_values_list() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("a")));
        dict.setitem(PyKey::Int(2), PyValue::Str(Arc::from("b")));

        let values = dict.values_list();
        if let PyValue::List(list) = values {
            assert_eq!(list.len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_dict_items_list() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("a")));

        let items = dict.items_list();
        if let PyValue::List(list) = items {
            assert_eq!(list.len(), 1);
            // Check that items are tuples
            let item = list.getitem(0).unwrap();
            if let PyValue::Tuple(tuple) = item {
                assert_eq!(tuple.len(), 2);
            } else {
                panic!("Expected Tuple");
            }
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_dict_get_with_default() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("a")));

        // Key exists
        let value = dict.get_with_default(&PyKey::Int(1), PyValue::None);
        if let PyValue::Str(s) = value {
            assert_eq!(&*s, "a");
        } else {
            panic!("Expected Str");
        }

        // Key doesn't exist - returns default
        let value = dict.get_with_default(&PyKey::Int(999), PyValue::Int(42));
        if let PyValue::Int(i) = value {
            assert_eq!(i, 42);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_dict_pop_with_default() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Str(Arc::from("a")));

        // Pop existing key
        let value = dict.pop_with_default(&PyKey::Int(1), None).unwrap();
        if let PyValue::Str(s) = value {
            assert_eq!(&*s, "a");
        } else {
            panic!("Expected Str");
        }
        assert!(dict.is_empty());

        // Pop non-existing key with default
        let value = dict.pop_with_default(&PyKey::Int(999), Some(PyValue::Int(42))).unwrap();
        if let PyValue::Int(i) = value {
            assert_eq!(i, 42);
        } else {
            panic!("Expected Int");
        }

        // Pop non-existing key without default - should error
        let result = dict.pop_with_default(&PyKey::Int(999), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_dict_update_from() {
        let dict1 = PyDict::new();
        dict1.setitem(PyKey::Int(1), PyValue::Int(1));

        let dict2 = PyDict::new();
        dict2.setitem(PyKey::Int(2), PyValue::Int(2));
        dict2.setitem(PyKey::Int(1), PyValue::Int(100)); // Override

        dict1.update_from(&dict2);
        assert_eq!(dict1.len(), 2);

        // Check that key 1 was overridden
        let value = dict1.getitem(&PyKey::Int(1)).unwrap();
        if let PyValue::Int(i) = value {
            assert_eq!(i, 100);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_dict_clear() {
        let dict = PyDict::new();
        dict.setitem(PyKey::Int(1), PyValue::Int(1));
        dict.setitem(PyKey::Int(2), PyValue::Int(2));

        assert_eq!(dict.len(), 2);
        dict.clear();
        assert!(dict.is_empty());
    }
}
