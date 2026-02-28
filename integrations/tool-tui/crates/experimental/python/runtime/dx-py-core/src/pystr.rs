//! PyStr - Python string type

use crate::error::{RuntimeError, RuntimeResult};
use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use std::sync::Arc;

/// Python string object
///
/// Immutable UTF-8 string with interning support
pub struct PyStr {
    /// Object header
    pub header: PyObjectHeader,
    /// String data (Arc for cheap cloning)
    data: Arc<str>,
    /// Cached hash
    hash: u64,
}

impl PyStr {
    /// Create a new string
    pub fn new(s: impl Into<String>) -> Self {
        let s: String = s.into();
        let hash = Self::compute_hash(&s);
        Self {
            header: PyObjectHeader::new(TypeTag::Str, ObjectFlags::IMMUTABLE),
            data: Arc::from(s),
            hash,
        }
    }

    /// Create from Arc<str> (zero-copy)
    pub fn from_arc(s: Arc<str>) -> Self {
        let hash = Self::compute_hash(&s);
        Self {
            header: PyObjectHeader::new(TypeTag::Str, ObjectFlags::IMMUTABLE),
            data: s,
            hash,
        }
    }

    /// Compute hash using FNV-1a
    fn compute_hash(s: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in s.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Get the string data
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.data
    }

    /// Get the length in characters
    #[inline]
    pub fn len(&self) -> usize {
        self.data.chars().count()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the byte length
    #[inline]
    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    /// Get cached hash
    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Concatenate two strings
    pub fn concat(&self, other: &PyStr) -> PyStr {
        let mut s = String::with_capacity(self.data.len() + other.data.len());
        s.push_str(&self.data);
        s.push_str(&other.data);
        PyStr::new(s)
    }

    /// Repeat string n times
    pub fn repeat(&self, n: usize) -> PyStr {
        PyStr::new(self.data.repeat(n))
    }

    /// Get character at index
    pub fn getitem(&self, index: i64) -> RuntimeResult<PyStr> {
        let len = self.len() as i64;
        let idx = if index < 0 { len + index } else { index };

        if idx < 0 || idx >= len {
            return Err(RuntimeError::index_error(index, len as usize));
        }

        self.data
            .chars()
            .nth(idx as usize)
            .map(|c| PyStr::new(c.to_string()))
            .ok_or_else(|| RuntimeError::index_error(index, len as usize))
    }

    /// Slice string
    pub fn slice(&self, start: Option<i64>, end: Option<i64>) -> PyStr {
        let len = self.len() as i64;

        let start = match start {
            Some(s) if s < 0 => (len + s).max(0) as usize,
            Some(s) => (s as usize).min(len as usize),
            None => 0,
        };

        let end = match end {
            Some(e) if e < 0 => (len + e).max(0) as usize,
            Some(e) => (e as usize).min(len as usize),
            None => len as usize,
        };

        if start >= end {
            return PyStr::new("");
        }

        let s: String = self.data.chars().skip(start).take(end - start).collect();
        PyStr::new(s)
    }

    /// Check if contains substring
    pub fn contains(&self, needle: &PyStr) -> bool {
        self.data.contains(needle.as_str())
    }

    /// Find substring
    pub fn find(&self, needle: &PyStr) -> Option<usize> {
        self.data.find(needle.as_str()).map(|pos| {
            // Convert byte position to char position
            self.data[..pos].chars().count()
        })
    }

    /// Find substring, returning -1 if not found (Python-compatible)
    pub fn find_index(&self, needle: &PyStr) -> i64 {
        match self.data.find(needle.as_str()) {
            Some(pos) => self.data[..pos].chars().count() as i64,
            None => -1,
        }
    }

    /// Count occurrences
    pub fn count(&self, needle: &PyStr) -> usize {
        self.data.matches(needle.as_str()).count()
    }

    /// Convert to uppercase
    pub fn upper(&self) -> PyStr {
        PyStr::new(self.data.to_uppercase())
    }

    /// Convert to lowercase
    pub fn lower(&self) -> PyStr {
        PyStr::new(self.data.to_lowercase())
    }

    /// Strip whitespace
    pub fn strip(&self) -> PyStr {
        PyStr::new(self.data.trim())
    }

    /// Strip left whitespace
    pub fn lstrip(&self) -> PyStr {
        PyStr::new(self.data.trim_start())
    }

    /// Strip right whitespace
    pub fn rstrip(&self) -> PyStr {
        PyStr::new(self.data.trim_end())
    }

    /// Split by separator
    /// If sep is None, splits on whitespace and removes empty strings
    pub fn split(&self, sep: Option<&PyStr>) -> Vec<PyStr> {
        match sep {
            Some(s) => self.data.split(s.as_str()).map(PyStr::new).collect(),
            None => {
                // Python's split() with no args splits on whitespace and removes empty strings
                self.data.split_whitespace().map(PyStr::new).collect()
            }
        }
    }

    /// Split by separator (legacy method for compatibility)
    pub fn split_sep(&self, sep: &PyStr) -> Vec<PyStr> {
        self.data.split(sep.as_str()).map(PyStr::new).collect()
    }

    /// Join strings
    pub fn join(&self, items: &[PyStr]) -> PyStr {
        let parts: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
        PyStr::new(parts.join(self.as_str()))
    }

    /// Replace occurrences
    pub fn replace(&self, old: &PyStr, new: &PyStr) -> PyStr {
        PyStr::new(self.data.replace(old.as_str(), new.as_str()))
    }

    /// Replace occurrences with optional count limit
    pub fn replace_count(&self, old: &PyStr, new: &PyStr, count: Option<usize>) -> PyStr {
        match count {
            Some(n) => PyStr::new(self.data.replacen(old.as_str(), new.as_str(), n)),
            None => PyStr::new(self.data.replace(old.as_str(), new.as_str())),
        }
    }

    /// Check if starts with prefix
    pub fn startswith(&self, prefix: &PyStr) -> bool {
        self.data.starts_with(prefix.as_str())
    }

    /// Check if ends with suffix
    pub fn endswith(&self, suffix: &PyStr) -> bool {
        self.data.ends_with(suffix.as_str())
    }

    /// Check if all characters are alphabetic
    pub fn isalpha(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_alphabetic())
    }

    /// Check if all characters are digits
    pub fn isdigit(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_ascii_digit())
    }

    /// Check if all characters are alphanumeric
    pub fn isalnum(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_alphanumeric())
    }

    /// Check if all characters are whitespace
    pub fn isspace(&self) -> bool {
        !self.data.is_empty() && self.data.chars().all(|c| c.is_whitespace())
    }

    /// Compare strings
    pub fn cmp(&self, other: &PyStr) -> std::cmp::Ordering {
        self.data.cmp(&other.data)
    }

    /// Check equality
    pub fn eq(&self, other: &PyStr) -> bool {
        // Fast path: compare hashes first
        if self.hash != other.hash {
            return false;
        }
        self.data == other.data
    }

    /// Convert to bool
    pub fn to_bool(&self) -> bool {
        !self.data.is_empty()
    }
}

impl std::fmt::Display for PyStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl std::fmt::Debug for PyStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PyStr({:?})", self.data)
    }
}

impl Clone for PyStr {
    fn clone(&self) -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::Str, ObjectFlags::IMMUTABLE),
            data: Arc::clone(&self.data),
            hash: self.hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_creation() {
        let s = PyStr::new("hello");
        assert_eq!(s.as_str(), "hello");
        assert_eq!(s.len(), 5);
        assert_eq!(s.header.type_tag(), TypeTag::Str);
    }

    #[test]
    fn test_str_concat() {
        let a = PyStr::new("hello");
        let b = PyStr::new(" world");
        let c = a.concat(&b);
        assert_eq!(c.as_str(), "hello world");
    }

    #[test]
    fn test_str_slice() {
        let s = PyStr::new("hello world");
        assert_eq!(s.slice(Some(0), Some(5)).as_str(), "hello");
        assert_eq!(s.slice(Some(6), None).as_str(), "world");
        assert_eq!(s.slice(Some(-5), None).as_str(), "world");
    }

    #[test]
    fn test_str_methods() {
        let s = PyStr::new("  Hello World  ");
        assert_eq!(s.strip().as_str(), "Hello World");
        assert_eq!(s.strip().upper().as_str(), "HELLO WORLD");
        assert_eq!(s.strip().lower().as_str(), "hello world");
    }

    #[test]
    fn test_str_find() {
        let s = PyStr::new("hello world");
        let needle = PyStr::new("world");
        assert_eq!(s.find(&needle), Some(6));
    }

    #[test]
    fn test_str_split_join() {
        let s = PyStr::new("a,b,c");
        let sep = PyStr::new(",");
        let parts = s.split(Some(&sep));
        assert_eq!(parts.len(), 3);

        let joined = sep.join(&parts);
        assert_eq!(joined.as_str(), "a,b,c");
    }

    #[test]
    fn test_str_split_whitespace() {
        let s = PyStr::new("  hello   world  ");
        let parts = s.split(None);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].as_str(), "hello");
        assert_eq!(parts[1].as_str(), "world");
    }

    #[test]
    fn test_str_find_index() {
        let s = PyStr::new("hello world");
        let needle = PyStr::new("world");
        let not_found = PyStr::new("xyz");
        assert_eq!(s.find_index(&needle), 6);
        assert_eq!(s.find_index(&not_found), -1);
    }

    #[test]
    fn test_str_replace_count() {
        let s = PyStr::new("aaa");
        let old = PyStr::new("a");
        let new = PyStr::new("b");
        assert_eq!(s.replace_count(&old, &new, Some(2)).as_str(), "bba");
        assert_eq!(s.replace_count(&old, &new, None).as_str(), "bbb");
    }

    #[test]
    fn test_str_split_edge_cases() {
        // Empty string
        let empty = PyStr::new("");
        let sep = PyStr::new(",");
        let parts = empty.split(Some(&sep));
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].as_str(), "");

        // No matches
        let s = PyStr::new("hello");
        let parts = s.split(Some(&sep));
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].as_str(), "hello");

        // Consecutive separators
        let s = PyStr::new("a,,b");
        let parts = s.split(Some(&sep));
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].as_str(), "a");
        assert_eq!(parts[1].as_str(), "");
        assert_eq!(parts[2].as_str(), "b");
    }

    #[test]
    fn test_str_join_edge_cases() {
        let sep = PyStr::new(",");
        
        // Empty list
        let empty: Vec<PyStr> = vec![];
        assert_eq!(sep.join(&empty).as_str(), "");

        // Single item
        let single = vec![PyStr::new("hello")];
        assert_eq!(sep.join(&single).as_str(), "hello");

        // Empty separator
        let empty_sep = PyStr::new("");
        let items = vec![PyStr::new("a"), PyStr::new("b"), PyStr::new("c")];
        assert_eq!(empty_sep.join(&items).as_str(), "abc");
    }

    #[test]
    fn test_str_startswith_endswith() {
        let s = PyStr::new("hello world");
        
        // startswith
        assert!(s.startswith(&PyStr::new("hello")));
        assert!(s.startswith(&PyStr::new("")));
        assert!(!s.startswith(&PyStr::new("world")));
        
        // endswith
        assert!(s.endswith(&PyStr::new("world")));
        assert!(s.endswith(&PyStr::new("")));
        assert!(!s.endswith(&PyStr::new("hello")));
    }

    #[test]
    fn test_str_hash_equality() {
        let a = PyStr::new("hello");
        let b = PyStr::new("hello");
        let c = PyStr::new("world");

        assert!(a.eq(&b));
        assert!(!a.eq(&c));
        assert_eq!(a.hash(), b.hash());
    }
}
