//! Core runtime integration tests

use dx_py_core::*;

#[test]
fn test_pyint_arithmetic() {
    let a = PyInt::new(10);
    let b = PyInt::new(20);
    
    // Addition
    let sum = a.add(&b);
    assert_eq!(sum.value(), 30);
    
    // Subtraction
    let diff = b.sub(&a);
    assert_eq!(diff.value(), 10);
    
    // Multiplication
    let prod = a.mul(&b);
    assert_eq!(prod.value(), 200);
}

#[test]
fn test_pystr_operations() {
    let s = PyStr::new("hello world");
    
    // Length
    assert_eq!(s.len(), 11);
    
    // Find
    assert_eq!(s.find("world"), Some(6));
    assert_eq!(s.find("xyz"), None);
    
    // Case conversion
    assert_eq!(s.upper().as_str(), "HELLO WORLD");
    assert_eq!(s.lower().as_str(), "hello world");
}

#[test]
fn test_pylist_operations() {
    let mut list = PyList::new();
    
    // Append
    list.append(PyInt::new(1).into());
    list.append(PyInt::new(2).into());
    list.append(PyInt::new(3).into());
    
    assert_eq!(list.len(), 3);
    
    // Get
    let first = list.get(0);
    assert!(first.is_some());
}

#[test]
fn test_pydict_operations() {
    let mut dict = PyDict::new();
    
    // Set
    dict.set("key1".into(), PyInt::new(100).into());
    dict.set("key2".into(), PyStr::new("value").into());
    
    assert_eq!(dict.len(), 2);
    
    // Get
    assert!(dict.get(&"key1".into()).is_some());
    assert!(dict.get(&"key3".into()).is_none());
    
    // Contains
    assert!(dict.contains(&"key1".into()));
    assert!(!dict.contains(&"key3".into()));
}

#[test]
fn test_pytuple_operations() {
    let tuple = PyTuple::from_vec(vec![
        PyInt::new(1).into(),
        PyInt::new(2).into(),
        PyInt::new(3).into(),
    ]);
    
    assert_eq!(tuple.len(), 3);
    assert!(tuple.get(0).is_some());
    assert!(tuple.get(10).is_none());
}

#[test]
fn test_pyframe_creation() {
    let frame = PyFrame::new("test_function".to_string(), 10);
    
    assert_eq!(frame.function_name(), "test_function");
    assert_eq!(frame.max_locals(), 10);
}

#[test]
fn test_error_types() {
    // Type error
    let err = RuntimeError::type_error("int", "str");
    assert_eq!(err.exception_name(), "TypeError");
    
    // Index error
    let err = RuntimeError::index_error(10, 5);
    assert_eq!(err.exception_name(), "IndexError");
    
    // Key error
    let err = RuntimeError::key_error("missing");
    assert_eq!(err.exception_name(), "KeyError");
    
    // Zero division
    let err = RuntimeError::ZeroDivisionError;
    assert_eq!(err.exception_name(), "ZeroDivisionError");
}

#[test]
fn test_line_table() {
    let table = LineTable::from_pairs(&[(0, 1), (10, 2), (20, 3)]);
    
    assert_eq!(table.get_line(0), Some(1));
    assert_eq!(table.get_line(5), Some(1));
    assert_eq!(table.get_line(10), Some(2));
    assert_eq!(table.get_line(15), Some(2));
    assert_eq!(table.get_line(20), Some(3));
}

#[test]
fn test_traceback() {
    let mut tb = Traceback::new();
    
    tb.push_frame(TracebackFrame::new(
        "test.py".to_string(),
        "test_func".to_string(),
        10,
    ));
    
    tb.push_frame(TracebackFrame::new(
        "test.py".to_string(),
        "main".to_string(),
        5,
    ));
    
    assert_eq!(tb.frames().len(), 2);
}
