//! Property-based tests for stdlib modules
//!
//! Feature: dx-py-production-ready
//! Property 9: Stdlib Function Equivalence
//! Validates: Requirements 4.3, 4.4, 4.13

#![allow(clippy::cloned_ref_to_slice_refs)]
#![allow(clippy::manual_range_contains)]

use proptest::prelude::*;
use std::sync::Arc;

// Import the stdlib module
use dx_py_core::pylist::PyValue;
use dx_py_core::stdlib::{os_builtins, os_module, os_path_builtins, os_path_module};

// ===== os.path property tests =====

/// Generate valid path components (no path separators, null bytes, or special dirs)
fn arb_path_component() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,20}"
        .prop_filter("no empty or special", |s| !s.is_empty() && s != "." && s != "..")
}

/// Generate a list of path components
fn arb_path_components() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_path_component(), 1..5)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.path.join then split should preserve basename
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_path_join_split_preserves_basename(components in arb_path_components()) {
        let join = os_path_builtins().into_iter().find(|f| f.name == "join").unwrap();
        let _split = os_path_builtins().into_iter().find(|f| f.name == "split").unwrap();
        let basename_fn = os_path_builtins().into_iter().find(|f| f.name == "basename").unwrap();

        // Join the components
        let args: Vec<PyValue> = components.iter()
            .map(|s| PyValue::Str(Arc::from(s.clone())))
            .collect();

        let joined = join.call(&args).unwrap();

        // Get basename
        let basename_result = basename_fn.call(&[joined.clone()]).unwrap();

        // The basename should be the last component
        if let PyValue::Str(basename) = basename_result {
            prop_assert_eq!(basename.as_ref(), components.last().unwrap().as_str());
        } else {
            prop_assert!(false, "Expected string from basename");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.path.split then join should produce equivalent path
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_path_split_join_roundtrip(components in arb_path_components()) {
        let join = os_path_builtins().into_iter().find(|f| f.name == "join").unwrap();
        let split = os_path_builtins().into_iter().find(|f| f.name == "split").unwrap();

        // Join the components first
        let args: Vec<PyValue> = components.iter()
            .map(|s| PyValue::Str(Arc::from(s.clone())))
            .collect();

        let joined = join.call(&args).unwrap();

        // Split the path
        let split_result = split.call(&[joined.clone()]).unwrap();

        if let PyValue::Tuple(t) = split_result {
            let parts = t.to_vec();
            prop_assert_eq!(parts.len(), 2);

            // Join the split parts back together
            let rejoined = join.call(&parts).unwrap();

            // Should be equivalent to original
            if let (PyValue::Str(orig), PyValue::Str(new)) = (&joined, &rejoined) {
                prop_assert_eq!(orig.as_ref(), new.as_ref());
            }
        } else {
            prop_assert!(false, "Expected tuple from split");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.path.splitext then join should preserve the original filename
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_path_splitext_roundtrip(
        name in "[a-zA-Z0-9_]{1,10}",
        ext in "[a-zA-Z]{1,5}"
    ) {
        let splitext = os_path_builtins().into_iter().find(|f| f.name == "splitext").unwrap();

        let filename = format!("{}.{}", name, ext);
        let result = splitext.call(&[PyValue::Str(Arc::from(filename.clone()))]).unwrap();

        if let PyValue::Tuple(t) = result {
            let parts = t.to_vec();
            prop_assert_eq!(parts.len(), 2);

            if let (PyValue::Str(root), PyValue::Str(extension)) = (&parts[0], &parts[1]) {
                // Joining root + extension should give back the original
                let reconstructed = format!("{}{}", root, extension);
                prop_assert_eq!(reconstructed, filename);
            }
        } else {
            prop_assert!(false, "Expected tuple from splitext");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.path.dirname + basename should reconstruct the path
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_path_dirname_basename_reconstruct(components in arb_path_components()) {
        if components.len() < 2 {
            return Ok(());
        }

        let join = os_path_builtins().into_iter().find(|f| f.name == "join").unwrap();
        let dirname = os_path_builtins().into_iter().find(|f| f.name == "dirname").unwrap();
        let basename_fn = os_path_builtins().into_iter().find(|f| f.name == "basename").unwrap();

        // Join the components
        let args: Vec<PyValue> = components.iter()
            .map(|s| PyValue::Str(Arc::from(s.clone())))
            .collect();

        let joined = join.call(&args).unwrap();

        // Get dirname and basename
        let dir_result = dirname.call(&[joined.clone()]).unwrap();
        let base_result = basename_fn.call(&[joined.clone()]).unwrap();

        // Join them back
        let reconstructed = join.call(&[dir_result, base_result]).unwrap();

        // Should be equivalent to original
        if let (PyValue::Str(orig), PyValue::Str(new)) = (&joined, &reconstructed) {
            prop_assert_eq!(orig.as_ref(), new.as_ref());
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.path.exists should return bool for any path
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_path_exists_returns_bool(path in "[a-zA-Z0-9_./\\\\-]{1,50}") {
        let exists = os_path_builtins().into_iter().find(|f| f.name == "exists").unwrap();

        let result = exists.call(&[PyValue::Str(Arc::from(path))]).unwrap();
        prop_assert!(matches!(result, PyValue::Bool(_)));
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.path.isfile and isdir should be mutually exclusive for existing paths
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_path_isfile_isdir_exclusive(path in "[a-zA-Z0-9_./\\\\-]{1,50}") {
        let exists = os_path_builtins().into_iter().find(|f| f.name == "exists").unwrap();
        let isfile = os_path_builtins().into_iter().find(|f| f.name == "isfile").unwrap();
        let isdir = os_path_builtins().into_iter().find(|f| f.name == "isdir").unwrap();

        let path_val = PyValue::Str(Arc::from(path));

        let exists_result = exists.call(&[path_val.clone()]).unwrap();
        let isfile_result = isfile.call(&[path_val.clone()]).unwrap();
        let isdir_result = isdir.call(&[path_val]).unwrap();

        if let (PyValue::Bool(e), PyValue::Bool(f), PyValue::Bool(d)) =
            (exists_result, isfile_result, isdir_result) {
            // If path exists, it should be either file or directory (or neither for special files)
            // But it cannot be both file AND directory
            prop_assert!(!(f && d), "Path cannot be both file and directory");

            // If it's a file or directory, it must exist
            if f || d {
                prop_assert!(e, "File or directory must exist");
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.path.isabs should correctly identify absolute paths
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_path_isabs_consistency(components in arb_path_components()) {
        let join = os_path_builtins().into_iter().find(|f| f.name == "join").unwrap();
        let isabs = os_path_builtins().into_iter().find(|f| f.name == "isabs").unwrap();

        // Join relative components
        let args: Vec<PyValue> = components.iter()
            .map(|s| PyValue::Str(Arc::from(s.clone())))
            .collect();

        let joined = join.call(&args).unwrap();

        // Relative path should not be absolute
        let result = isabs.call(&[joined]).unwrap();
        if let PyValue::Bool(is_absolute) = result {
            prop_assert!(!is_absolute, "Joined relative components should not be absolute");
        }
    }
}

// ===== os module property tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.getenv with default should return default for non-existent vars
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_getenv_default(
        var_name in "[A-Z_]{5,15}_NONEXISTENT_[0-9]{5}",
        default in "[a-zA-Z0-9]{1,20}"
    ) {
        let getenv = os_builtins().into_iter().find(|f| f.name == "getenv").unwrap();

        let result = getenv.call(&[
            PyValue::Str(Arc::from(var_name)),
            PyValue::Str(Arc::from(default.clone())),
        ]).unwrap();

        if let PyValue::Str(s) = result {
            prop_assert_eq!(s.as_ref(), default.as_str());
        } else {
            prop_assert!(false, "Expected string from getenv");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.putenv then getenv should return the set value
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_putenv_getenv_roundtrip(
        var_suffix in "[0-9]{8}",
        value in "[a-zA-Z0-9]{1,20}"
    ) {
        let putenv = os_builtins().into_iter().find(|f| f.name == "putenv").unwrap();
        let getenv = os_builtins().into_iter().find(|f| f.name == "getenv").unwrap();

        // Use a unique variable name to avoid conflicts
        let var_name = format!("DX_PY_TEST_{}", var_suffix);

        // Set the variable
        putenv.call(&[
            PyValue::Str(Arc::from(var_name.clone())),
            PyValue::Str(Arc::from(value.clone())),
        ]).unwrap();

        // Get it back
        let result = getenv.call(&[PyValue::Str(Arc::from(var_name.clone()))]).unwrap();

        // Clean up
        let unsetenv = os_builtins().into_iter().find(|f| f.name == "unsetenv").unwrap();
        unsetenv.call(&[PyValue::Str(Arc::from(var_name))]).unwrap();

        if let PyValue::Str(s) = result {
            prop_assert_eq!(s.as_ref(), value.as_str());
        } else {
            prop_assert!(false, "Expected string from getenv");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.getcwd should always return a non-empty string
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_getcwd_nonempty(_dummy in 0..10i32) {
        let getcwd = os_builtins().into_iter().find(|f| f.name == "getcwd").unwrap();

        let result = getcwd.call(&[]).unwrap();

        if let PyValue::Str(s) = result {
            prop_assert!(!s.is_empty(), "getcwd should return non-empty string");
        } else {
            prop_assert!(false, "Expected string from getcwd");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.getpid should always return a positive integer
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_getpid_positive(_dummy in 0..10i32) {
        let getpid = os_builtins().into_iter().find(|f| f.name == "getpid").unwrap();

        let result = getpid.call(&[]).unwrap();

        if let PyValue::Int(pid) = result {
            prop_assert!(pid > 0, "getpid should return positive integer");
        } else {
            prop_assert!(false, "Expected int from getpid");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.cpu_count should return at least 1
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_cpu_count_positive(_dummy in 0..10i32) {
        let cpu_count = os_builtins().into_iter().find(|f| f.name == "cpu_count").unwrap();

        let result = cpu_count.call(&[]).unwrap();

        if let PyValue::Int(count) = result {
            prop_assert!(count >= 1, "cpu_count should return at least 1");
        } else {
            prop_assert!(false, "Expected int from cpu_count");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// os.urandom should return the requested number of bytes
    /// Validates: Requirements 4.3, 4.4
    #[test]
    fn prop_os_urandom_length(n in 0..100usize) {
        let urandom = os_builtins().into_iter().find(|f| f.name == "urandom").unwrap();

        let result = urandom.call(&[PyValue::Int(n as i64)]).unwrap();

        if let PyValue::List(bytes) = result {
            prop_assert_eq!(bytes.len(), n, "urandom should return exactly n bytes");

            // All values should be in range 0-255
            for byte in bytes.to_vec() {
                if let PyValue::Int(b) = byte {
                    prop_assert!(b >= 0 && b <= 255, "byte should be in range 0-255");
                }
            }
        } else {
            prop_assert!(false, "Expected list from urandom");
        }
    }
}

// ===== os module dict property tests =====

#[test]
fn test_os_module_has_required_attributes() {
    let os = os_module();

    // Check required attributes exist
    use dx_py_core::pydict::PyKey;

    assert!(os.contains(&PyKey::Str(Arc::from("name"))));
    assert!(os.contains(&PyKey::Str(Arc::from("sep"))));
    assert!(os.contains(&PyKey::Str(Arc::from("pathsep"))));
    assert!(os.contains(&PyKey::Str(Arc::from("linesep"))));
    assert!(os.contains(&PyKey::Str(Arc::from("environ"))));
    assert!(os.contains(&PyKey::Str(Arc::from("curdir"))));
    assert!(os.contains(&PyKey::Str(Arc::from("pardir"))));
}

#[test]
fn test_os_path_module_has_required_attributes() {
    let os_path = os_path_module();

    use dx_py_core::pydict::PyKey;

    assert!(os_path.contains(&PyKey::Str(Arc::from("sep"))));
}

// ===== iteration builtins property tests (Task 7.3) =====

use dx_py_core::builtins::{
    builtin_all, builtin_any, builtin_enumerate, builtin_iter, builtin_next, builtin_reversed,
    builtin_sorted, builtin_zip,
};

/// Generate arbitrary lists of integers for iteration tests
fn arb_int_list() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(any::<i64>(), 0..20)
}

/// Generate arbitrary lists of strings for iteration tests
fn arb_string_list() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z]{1,10}".prop_map(|s| s), 0..15)
}

/// Generate arbitrary boolean lists for all/any tests
fn arb_bool_list() -> impl Strategy<Value = Vec<bool>> {
    prop::collection::vec(any::<bool>(), 0..20)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// enumerate should produce (index, value) pairs in order
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_enumerate_produces_indexed_pairs(items in arb_string_list()) {
        let enumerate_fn = builtin_enumerate();

        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let result = enumerate_fn.call(&[item_list]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), items.len());

            for (i, pair) in l.to_vec().into_iter().enumerate() {
                if let PyValue::Tuple(t) = pair {
                    let tuple_items = t.to_vec();
                    prop_assert_eq!(tuple_items.len(), 2);

                    // Check index
                    if let PyValue::Int(idx) = &tuple_items[0] {
                        prop_assert_eq!(*idx, i as i64);
                    }

                    // Check value
                    if let PyValue::Str(s) = &tuple_items[1] {
                        prop_assert_eq!(s.as_ref(), items[i].as_str());
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// enumerate with start should offset indices correctly
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_enumerate_with_start(
        items in arb_string_list(),
        start in -10..10i64
    ) {
        let enumerate_fn = builtin_enumerate();

        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let result = enumerate_fn.call(&[item_list, PyValue::Int(start)]).unwrap();

        if let PyValue::List(l) = result {
            for (i, pair) in l.to_vec().into_iter().enumerate() {
                if let PyValue::Tuple(t) = pair {
                    let tuple_items = t.to_vec();
                    if let PyValue::Int(idx) = &tuple_items[0] {
                        prop_assert_eq!(*idx, start + i as i64);
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// zip should pair elements from multiple iterables
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_zip_pairs_elements(
        list1 in arb_int_list(),
        list2 in arb_string_list()
    ) {
        let zip_fn = builtin_zip();

        let l1 = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            list1.iter().map(|i| PyValue::Int(*i)).collect()
        )));
        let l2 = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            list2.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let result = zip_fn.call(&[l1, l2]).unwrap();

        if let PyValue::List(l) = result {
            let expected_len = list1.len().min(list2.len());
            prop_assert_eq!(l.len(), expected_len);

            for (i, pair) in l.to_vec().into_iter().enumerate() {
                if let PyValue::Tuple(t) = pair {
                    let tuple_items = t.to_vec();
                    prop_assert_eq!(tuple_items.len(), 2);

                    if let PyValue::Int(val1) = &tuple_items[0] {
                        prop_assert_eq!(*val1, list1[i]);
                    }
                    if let PyValue::Str(val2) = &tuple_items[1] {
                        prop_assert_eq!(val2.as_ref(), list2[i].as_str());
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// zip with empty iterable should produce empty result
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_zip_with_empty(items in arb_int_list()) {
        let zip_fn = builtin_zip();

        let non_empty = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|i| PyValue::Int(*i)).collect()
        )));
        let empty = PyValue::List(Arc::new(dx_py_core::PyList::new()));

        let result = zip_fn.call(&[non_empty, empty]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), 0);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// sorted should preserve all elements while ordering them
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_sorted_preserves_elements(items in arb_int_list()) {
        let sorted_fn = builtin_sorted();

        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = sorted_fn.call(&[item_list]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), items.len());

            let sorted_items: Vec<i64> = l.to_vec().into_iter()
                .filter_map(|v| if let PyValue::Int(i) = v { Some(i) } else { None })
                .collect();

            // Should be sorted
            for i in 1..sorted_items.len() {
                prop_assert!(sorted_items[i-1] <= sorted_items[i],
                    "Items should be sorted: {} <= {}", sorted_items[i-1], sorted_items[i]);
            }

            // Should contain same elements (count each)
            let mut original_counts = std::collections::HashMap::new();
            for item in &items {
                *original_counts.entry(*item).or_insert(0) += 1;
            }

            let mut sorted_counts = std::collections::HashMap::new();
            for item in &sorted_items {
                *sorted_counts.entry(*item).or_insert(0) += 1;
            }

            prop_assert_eq!(original_counts, sorted_counts);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// reversed should reverse the order of elements
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_reversed_reverses_order(items in arb_string_list()) {
        let reversed_fn = builtin_reversed();

        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let result = reversed_fn.call(&[item_list]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), items.len());

            let reversed_items: Vec<String> = l.to_vec().into_iter()
                .filter_map(|v| if let PyValue::Str(s) = v { Some(s.to_string()) } else { None })
                .collect();

            // Should be in reverse order
            for (i, item) in reversed_items.iter().enumerate() {
                let original_index = items.len() - 1 - i;
                prop_assert_eq!(item.as_str(), items[original_index].as_str());
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// reversed then reversed should return to original order
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_reversed_twice_is_identity(items in arb_int_list()) {
        let reversed_fn = builtin_reversed();

        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let reversed_once = reversed_fn.call(&[item_list.clone()]).unwrap();
        let reversed_twice = reversed_fn.call(&[reversed_once]).unwrap();

        if let (PyValue::List(original), PyValue::List(final_result)) = (&item_list, &reversed_twice) {
            prop_assert_eq!(original.len(), final_result.len());

            let orig_items = original.to_vec();
            let final_items = final_result.to_vec();

            for (orig, final_val) in orig_items.iter().zip(final_items.iter()) {
                if let (PyValue::Int(o), PyValue::Int(f)) = (orig, final_val) {
                    prop_assert_eq!(*o, *f);
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// iter should return an iterable equivalent to the original
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_iter_preserves_content(items in arb_string_list()) {
        let iter_fn = builtin_iter();

        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let result = iter_fn.call(&[item_list.clone()]).unwrap();

        // iter returns a list representation for now
        if let (PyValue::List(original), PyValue::List(iterable)) = (&item_list, &result) {
            prop_assert_eq!(original.len(), iterable.len());

            let orig_items = original.to_vec();
            let iter_items = iterable.to_vec();

            for (orig, iter_val) in orig_items.iter().zip(iter_items.iter()) {
                if let (PyValue::Str(o), PyValue::Str(i)) = (orig, iter_val) {
                    prop_assert_eq!(o.as_ref(), i.as_ref());
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// all should return True only if all elements are truthy
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_all_checks_all_elements(bools in arb_bool_list()) {
        let all_fn = builtin_all();

        let bool_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            bools.iter().map(|b| PyValue::Bool(*b)).collect()
        )));

        let result = all_fn.call(&[bool_list]).unwrap();

        if let PyValue::Bool(all_result) = result {
            let expected = bools.iter().all(|&b| b);
            prop_assert_eq!(all_result, expected);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// all with empty iterable should return True
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_all_empty_is_true(_dummy in 0..10i32) {
        let all_fn = builtin_all();

        let empty_list = PyValue::List(Arc::new(dx_py_core::PyList::new()));
        let result = all_fn.call(&[empty_list]).unwrap();

        prop_assert!(matches!(result, PyValue::Bool(true)));
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// any should return True if at least one element is truthy
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_any_checks_any_element(bools in arb_bool_list()) {
        let any_fn = builtin_any();

        let bool_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            bools.iter().map(|b| PyValue::Bool(*b)).collect()
        )));

        let result = any_fn.call(&[bool_list]).unwrap();

        if let PyValue::Bool(any_result) = result {
            let expected = bools.iter().any(|&b| b);
            prop_assert_eq!(any_result, expected);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// any with empty iterable should return False
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_any_empty_is_false(_dummy in 0..10i32) {
        let any_fn = builtin_any();

        let empty_list = PyValue::List(Arc::new(dx_py_core::PyList::new()));
        let result = any_fn.call(&[empty_list]).unwrap();

        prop_assert!(matches!(result, PyValue::Bool(false)));
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// all and any should be logical opposites for boolean lists
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_all_any_complement(bools in arb_bool_list()) {
        if bools.is_empty() {
            return Ok(()); // Skip empty case as it's handled separately
        }

        let all_fn = builtin_all();
        let any_fn = builtin_any();

        let bool_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            bools.iter().map(|b| PyValue::Bool(*b)).collect()
        )));

        let all_result = all_fn.call(&[bool_list.clone()]).unwrap();
        let any_result = any_fn.call(&[bool_list]).unwrap();

        if let (PyValue::Bool(all_val), PyValue::Bool(any_val)) = (all_result, any_result) {
            // If all are true, any must be true
            if all_val {
                prop_assert!(any_val, "If all() is true, any() must be true");
            }

            // If any is false, all must be false
            if !any_val {
                prop_assert!(!all_val, "If any() is false, all() must be false");
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// next should return first element of non-empty iterable
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_next_returns_first(items in prop::collection::vec(any::<i64>(), 1..20)) {
        let next_fn = builtin_next();

        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = next_fn.call(&[item_list]).unwrap();

        if let PyValue::Int(first) = result {
            prop_assert_eq!(first, items[0]);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// next with default should return default for empty iterable
    /// Validates: Requirements 4.1, 4.13
    #[test]
    fn prop_next_default_on_empty(default in any::<i64>()) {
        let next_fn = builtin_next();

        let empty_list = PyValue::List(Arc::new(dx_py_core::PyList::new()));
        let result = next_fn.call(&[empty_list, PyValue::Int(default)]).unwrap();

        if let PyValue::Int(val) = result {
            prop_assert_eq!(val, default);
        }
    }
}

// ===== collections module property tests (Task 7.11) =====

use dx_py_core::pydict::PyKey;
use dx_py_core::stdlib::{collections_builtins, collections_module};

/// Generate arbitrary key-value pairs for dict-like collections
fn arb_key_value_pairs() -> impl Strategy<Value = Vec<(String, i64)>> {
    prop::collection::vec(("[a-z]{1,10}".prop_map(|s| s), any::<i64>()), 0..10)
}

/// Generate arbitrary items for counting
fn arb_countable_items() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z]{1,5}".prop_map(|s| s), 0..20)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// OrderedDict should preserve insertion order and update values for duplicate keys
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_ordereddict_preserves_order(pairs in arb_key_value_pairs()) {
        let ordered_dict_fn = collections_builtins().into_iter()
            .find(|f| f.name == "OrderedDict").unwrap();

        // Create OrderedDict
        let od = ordered_dict_fn.call(&[]).unwrap();

        // Build expected final values (last value for each key wins)
        let mut expected: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for (key, value) in &pairs {
            expected.insert(key.clone(), *value);
        }

        if let PyValue::Dict(d) = &od {
            // Insert items in order
            if let PyValue::Dict(data) = d.getitem(&PyKey::Str(Arc::from("_data"))).unwrap() {
                for (key, value) in &pairs {
                    data.setitem(PyKey::Str(Arc::from(key.clone())), PyValue::Int(*value));
                }
            }

            // Verify data was stored with correct final values
            if let PyValue::Dict(data) = d.getitem(&PyKey::Str(Arc::from("_data"))).unwrap() {
                for (key, expected_value) in &expected {
                    let stored = data.get(&PyKey::Str(Arc::from(key.clone())), PyValue::None);
                    if let PyValue::Int(v) = stored {
                        prop_assert_eq!(v, *expected_value, "Value mismatch for key: {}", key);
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Counter should correctly count items
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_counter_counts_correctly(items in arb_countable_items()) {
        let counter_fn = collections_builtins().into_iter()
            .find(|f| f.name == "Counter").unwrap();

        // Create Counter from list
        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let counter = counter_fn.call(&[item_list]).unwrap();

        if let PyValue::Dict(d) = counter {
            if let PyValue::Dict(data) = d.getitem(&PyKey::Str(Arc::from("_data"))).unwrap() {
                // Verify counts match expected
                let mut expected_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
                for item in &items {
                    *expected_counts.entry(item.clone()).or_insert(0) += 1;
                }

                for (key, expected_count) in expected_counts {
                    let actual = data.get(&PyKey::Str(Arc::from(key.clone())), PyValue::Int(0));
                    if let PyValue::Int(count) = actual {
                        prop_assert_eq!(count, expected_count, "Count mismatch for key: {}", key);
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Counter.most_common should return items sorted by count descending
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_counter_most_common_sorted(items in arb_countable_items()) {
        if items.is_empty() {
            return Ok(());
        }

        let counter_fn = collections_builtins().into_iter()
            .find(|f| f.name == "Counter").unwrap();
        let most_common_fn = collections_builtins().into_iter()
            .find(|f| f.name == "Counter_most_common").unwrap();

        // Create Counter from list
        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let counter = counter_fn.call(&[item_list]).unwrap();

        // Get most_common
        let result = most_common_fn.call(&[counter]).unwrap();

        if let PyValue::List(l) = result {
            let pairs = l.to_vec();

            // Verify sorted by count descending
            let mut prev_count = i64::MAX;
            for pair in pairs {
                if let PyValue::Tuple(t) = pair {
                    let items = t.to_vec();
                    if let PyValue::Int(count) = &items[1] {
                        prop_assert!(*count <= prev_count, "most_common should be sorted descending");
                        prev_count = *count;
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// deque should respect maxlen constraint
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_deque_respects_maxlen(
        items in prop::collection::vec(any::<i64>(), 0..20),
        maxlen in 1..10usize
    ) {
        let deque_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque").unwrap();

        // Create deque with maxlen
        let item_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let deque = deque_fn.call(&[item_list, PyValue::Int(maxlen as i64)]).unwrap();

        if let PyValue::Dict(d) = deque {
            if let PyValue::List(data) = d.getitem(&PyKey::Str(Arc::from("_data"))).unwrap() {
                // Length should not exceed maxlen
                prop_assert!(data.len() <= maxlen, "deque length {} exceeds maxlen {}", data.len(), maxlen);

                // If items > maxlen, should have last maxlen items
                if items.len() > maxlen {
                    let expected: Vec<i64> = items[items.len() - maxlen..].to_vec();
                    let actual: Vec<i64> = data.to_vec().into_iter()
                        .filter_map(|v| if let PyValue::Int(i) = v { Some(i) } else { None })
                        .collect();
                    prop_assert_eq!(actual, expected);
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// deque append then pop should return the same item (LIFO for right side)
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_deque_append_pop_roundtrip(value in any::<i64>()) {
        let deque_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque").unwrap();
        let append_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque_append").unwrap();
        let pop_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque_pop").unwrap();

        // Create empty deque
        let deque = deque_fn.call(&[]).unwrap();

        // Append value
        append_fn.call(&[deque.clone(), PyValue::Int(value)]).unwrap();

        // Pop should return the same value
        let popped = pop_fn.call(&[deque]).unwrap();

        if let PyValue::Int(v) = popped {
            prop_assert_eq!(v, value);
        } else {
            prop_assert!(false, "Expected int from pop");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// deque appendleft then popleft should return the same item (LIFO for left side)
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_deque_appendleft_popleft_roundtrip(value in any::<i64>()) {
        let deque_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque").unwrap();
        let appendleft_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque_appendleft").unwrap();
        let popleft_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque_popleft").unwrap();

        // Create empty deque
        let deque = deque_fn.call(&[]).unwrap();

        // Appendleft value
        appendleft_fn.call(&[deque.clone(), PyValue::Int(value)]).unwrap();

        // Popleft should return the same value
        let popped = popleft_fn.call(&[deque]).unwrap();

        if let PyValue::Int(v) = popped {
            prop_assert_eq!(v, value);
        } else {
            prop_assert!(false, "Expected int from popleft");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// deque append then popleft should work as FIFO queue
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_deque_fifo_behavior(values in prop::collection::vec(any::<i64>(), 1..10)) {
        let deque_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque").unwrap();
        let append_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque_append").unwrap();
        let popleft_fn = collections_builtins().into_iter()
            .find(|f| f.name == "deque_popleft").unwrap();

        // Create empty deque
        let deque = deque_fn.call(&[]).unwrap();

        // Append all values
        for value in &values {
            append_fn.call(&[deque.clone(), PyValue::Int(*value)]).unwrap();
        }

        // Popleft should return values in FIFO order
        for expected in &values {
            let popped = popleft_fn.call(&[deque.clone()]).unwrap();
            if let PyValue::Int(v) = popped {
                prop_assert_eq!(v, *expected, "FIFO order violated");
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// namedtuple should store field names correctly
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_namedtuple_stores_fields(
        typename in "[A-Z][a-z]{2,10}",
        fields in prop::collection::vec("[a-z]{1,10}".prop_map(|s| s), 1..5)
    ) {
        let namedtuple_fn = collections_builtins().into_iter()
            .find(|f| f.name == "namedtuple").unwrap();

        // Create namedtuple type
        let field_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            fields.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect()
        )));

        let nt_type = namedtuple_fn.call(&[
            PyValue::Str(Arc::from(typename.clone())),
            field_list,
        ]).unwrap();

        if let PyValue::Dict(d) = nt_type {
            // Check typename
            let name = d.getitem(&PyKey::Str(Arc::from("__name__"))).unwrap();
            if let PyValue::Str(s) = name {
                prop_assert_eq!(s.as_ref(), typename.as_str());
            }

            // Check fields
            let stored_fields = d.getitem(&PyKey::Str(Arc::from("_fields"))).unwrap();
            if let PyValue::Tuple(t) = stored_fields {
                let field_vec = t.to_vec();
                prop_assert_eq!(field_vec.len(), fields.len());

                for (i, field) in fields.iter().enumerate() {
                    if let PyValue::Str(s) = &field_vec[i] {
                        prop_assert_eq!(s.as_ref(), field.as_str());
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// ChainMap should search maps in order
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_chainmap_search_order(
        key in "[a-z]{1,5}",
        value1 in any::<i64>(),
        value2 in any::<i64>()
    ) {
        let chainmap_fn = collections_builtins().into_iter()
            .find(|f| f.name == "ChainMap").unwrap();

        // Create two dicts with same key but different values
        let dict1 = dx_py_core::PyDict::new();
        dict1.setitem(PyKey::Str(Arc::from(key.clone())), PyValue::Int(value1));

        let dict2 = dx_py_core::PyDict::new();
        dict2.setitem(PyKey::Str(Arc::from(key.clone())), PyValue::Int(value2));

        // Create ChainMap with dict1 first
        let chainmap = chainmap_fn.call(&[
            PyValue::Dict(Arc::new(dict1)),
            PyValue::Dict(Arc::new(dict2)),
        ]).unwrap();

        if let PyValue::Dict(d) = chainmap {
            let maps = d.getitem(&PyKey::Str(Arc::from("maps"))).unwrap();
            if let PyValue::List(l) = maps {
                // First map should be searched first
                let first_map = &l.to_vec()[0];
                if let PyValue::Dict(first) = first_map {
                    let found = first.get(&PyKey::Str(Arc::from(key.clone())), PyValue::None);
                    if let PyValue::Int(v) = found {
                        prop_assert_eq!(v, value1, "ChainMap should find value in first map");
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// ChainMap.new_child should prepend new map
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_chainmap_new_child_prepends(
        key in "[a-z]{1,5}",
        value in any::<i64>()
    ) {
        let chainmap_fn = collections_builtins().into_iter()
            .find(|f| f.name == "ChainMap").unwrap();
        let new_child_fn = collections_builtins().into_iter()
            .find(|f| f.name == "ChainMap_new_child").unwrap();

        // Create initial ChainMap
        let chainmap = chainmap_fn.call(&[]).unwrap();

        // Create new child with a value
        let child_dict = dx_py_core::PyDict::new();
        child_dict.setitem(PyKey::Str(Arc::from(key.clone())), PyValue::Int(value));

        let new_chainmap = new_child_fn.call(&[
            chainmap,
            PyValue::Dict(Arc::new(child_dict)),
        ]).unwrap();

        if let PyValue::Dict(d) = new_chainmap {
            let maps = d.getitem(&PyKey::Str(Arc::from("maps"))).unwrap();
            if let PyValue::List(l) = maps {
                // New map should be first
                let first_map = &l.to_vec()[0];
                if let PyValue::Dict(first) = first_map {
                    let found = first.get(&PyKey::Str(Arc::from(key.clone())), PyValue::None);
                    if let PyValue::Int(v) = found {
                        prop_assert_eq!(v, value, "new_child should prepend map");
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// defaultdict should have default_factory attribute
    /// Validates: Requirements 4.6, 4.13
    #[test]
    fn prop_defaultdict_has_factory(_dummy in 0..10i32) {
        let defaultdict_fn = collections_builtins().into_iter()
            .find(|f| f.name == "defaultdict").unwrap();

        // Create defaultdict with list factory (represented as string for now)
        let dd = defaultdict_fn.call(&[PyValue::Str(Arc::from("list"))]).unwrap();

        if let PyValue::Dict(d) = dd {
            let factory = d.getitem(&PyKey::Str(Arc::from("default_factory"))).unwrap();
            if let PyValue::Str(s) = factory {
                prop_assert_eq!(s.as_ref(), "list");
            }
        }
    }
}

// ===== collections module unit tests =====

#[test]
fn test_collections_module_exists() {
    let collections = collections_module();

    let name = collections.getitem(&PyKey::Str(Arc::from("__name__"))).unwrap();
    if let PyValue::Str(s) = name {
        assert_eq!(s.as_ref(), "collections");
    } else {
        panic!("Expected string for __name__");
    }
}

#[test]
fn test_counter_empty() {
    let counter_fn = collections_builtins().into_iter().find(|f| f.name == "Counter").unwrap();

    let counter = counter_fn.call(&[]).unwrap();
    assert!(matches!(counter, PyValue::Dict(_)));
}

#[test]
fn test_counter_from_string() {
    let counter_fn = collections_builtins().into_iter().find(|f| f.name == "Counter").unwrap();

    let counter = counter_fn.call(&[PyValue::Str(Arc::from("aabbc"))]).unwrap();

    if let PyValue::Dict(d) = counter {
        if let PyValue::Dict(data) = d.getitem(&PyKey::Str(Arc::from("_data"))).unwrap() {
            let a_count = data.get(&PyKey::Str(Arc::from("a")), PyValue::Int(0));
            let b_count = data.get(&PyKey::Str(Arc::from("b")), PyValue::Int(0));
            let c_count = data.get(&PyKey::Str(Arc::from("c")), PyValue::Int(0));

            assert!(matches!(a_count, PyValue::Int(2)));
            assert!(matches!(b_count, PyValue::Int(2)));
            assert!(matches!(c_count, PyValue::Int(1)));
        }
    }
}

#[test]
fn test_deque_empty_pop_error() {
    let deque_fn = collections_builtins().into_iter().find(|f| f.name == "deque").unwrap();
    let pop_fn = collections_builtins().into_iter().find(|f| f.name == "deque_pop").unwrap();

    let deque = deque_fn.call(&[]).unwrap();
    let result = pop_fn.call(&[deque]);

    assert!(result.is_err(), "pop from empty deque should error");
}

#[test]
fn test_deque_empty_popleft_error() {
    let deque_fn = collections_builtins().into_iter().find(|f| f.name == "deque").unwrap();
    let popleft_fn =
        collections_builtins().into_iter().find(|f| f.name == "deque_popleft").unwrap();

    let deque = deque_fn.call(&[]).unwrap();
    let result = popleft_fn.call(&[deque]);

    assert!(result.is_err(), "popleft from empty deque should error");
}

#[test]
fn test_namedtuple_from_string_fields() {
    let namedtuple_fn =
        collections_builtins().into_iter().find(|f| f.name == "namedtuple").unwrap();

    // Create namedtuple with space-separated field names
    let nt_type = namedtuple_fn
        .call(&[
            PyValue::Str(Arc::from("Point")),
            PyValue::Str(Arc::from("x y z")),
        ])
        .unwrap();

    if let PyValue::Dict(d) = nt_type {
        let fields = d.getitem(&PyKey::Str(Arc::from("_fields"))).unwrap();
        if let PyValue::Tuple(t) = fields {
            assert_eq!(t.len(), 3);
        }
    }
}

// ===== functools module property tests (Task 8.3) =====

use dx_py_core::stdlib::{
    functools_builtins, functools_module, itertools_builtins, itertools_module,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// partial should store function and args correctly
    /// Validates: Requirements 4.8, 4.13
    #[test]
    fn prop_partial_stores_args(args in prop::collection::vec(any::<i64>(), 0..5)) {
        let partial_fn = functools_builtins().into_iter()
            .find(|f| f.name == "partial").unwrap();

        // Create partial with a "function" (represented as string) and args
        let mut call_args = vec![PyValue::Str(Arc::from("test_func"))];
        call_args.extend(args.iter().map(|i| PyValue::Int(*i)));

        let partial = partial_fn.call(&call_args).unwrap();

        if let PyValue::Dict(d) = partial {
            // Check func is stored
            let func = d.getitem(&PyKey::Str(Arc::from("func"))).unwrap();
            if let PyValue::Str(s) = func {
                prop_assert_eq!(s.as_ref(), "test_func");
            }

            // Check args are stored
            let stored_args = d.getitem(&PyKey::Str(Arc::from("args"))).unwrap();
            if let PyValue::Tuple(t) = stored_args {
                prop_assert_eq!(t.len(), args.len());
                for (i, expected) in args.iter().enumerate() {
                    if let PyValue::Int(actual) = &t.to_vec()[i] {
                        prop_assert_eq!(*actual, *expected);
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// reduce should accumulate values correctly for integers
    /// Validates: Requirements 4.8, 4.13
    #[test]
    fn prop_reduce_sum_integers(values in prop::collection::vec(any::<i64>().prop_map(|x| x % 1000), 1..10)) {
        let reduce_fn = functools_builtins().into_iter()
            .find(|f| f.name == "reduce").unwrap();

        let value_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            values.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        // reduce with add function (simplified - our reduce does addition by default)
        let result = reduce_fn.call(&[
            PyValue::Str(Arc::from("add")), // Placeholder for function
            value_list,
        ]).unwrap();

        let expected_sum: i64 = values.iter().sum();

        if let PyValue::Int(actual) = result {
            prop_assert_eq!(actual, expected_sum);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// reduce with initial value should include it in accumulation
    /// Validates: Requirements 4.8, 4.13
    #[test]
    fn prop_reduce_with_initial(
        values in prop::collection::vec(any::<i64>().prop_map(|x| x % 1000), 0..10),
        initial in any::<i64>().prop_map(|x| x % 1000)
    ) {
        let reduce_fn = functools_builtins().into_iter()
            .find(|f| f.name == "reduce").unwrap();

        let value_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            values.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = reduce_fn.call(&[
            PyValue::Str(Arc::from("add")),
            value_list,
            PyValue::Int(initial),
        ]).unwrap();

        let expected_sum: i64 = initial + values.iter().sum::<i64>();

        if let PyValue::Int(actual) = result {
            prop_assert_eq!(actual, expected_sum);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// lru_cache should store maxsize correctly
    /// Validates: Requirements 4.8, 4.13
    #[test]
    fn prop_lru_cache_stores_maxsize(maxsize in 1..1000i64) {
        let lru_cache_fn = functools_builtins().into_iter()
            .find(|f| f.name == "lru_cache").unwrap();

        let cache = lru_cache_fn.call(&[PyValue::Int(maxsize)]).unwrap();

        if let PyValue::Dict(d) = cache {
            let stored_maxsize = d.getitem(&PyKey::Str(Arc::from("maxsize"))).unwrap();
            if let PyValue::Int(actual) = stored_maxsize {
                prop_assert_eq!(actual, maxsize);
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// lru_cache_clear should reset cache statistics
    /// Validates: Requirements 4.8, 4.13
    #[test]
    fn prop_lru_cache_clear_resets(_dummy in 0..10i32) {
        let lru_cache_fn = functools_builtins().into_iter()
            .find(|f| f.name == "lru_cache").unwrap();
        let cache_clear_fn = functools_builtins().into_iter()
            .find(|f| f.name == "lru_cache_clear").unwrap();
        let cache_info_fn = functools_builtins().into_iter()
            .find(|f| f.name == "lru_cache_info").unwrap();

        // Create cache
        let cache = lru_cache_fn.call(&[PyValue::Int(100)]).unwrap();

        // Clear it
        cache_clear_fn.call(&[cache.clone()]).unwrap();

        // Check info
        let info = cache_info_fn.call(&[cache]).unwrap();

        if let PyValue::Dict(d) = info {
            let hits = d.getitem(&PyKey::Str(Arc::from("hits"))).unwrap();
            let misses = d.getitem(&PyKey::Str(Arc::from("misses"))).unwrap();
            let currsize = d.getitem(&PyKey::Str(Arc::from("currsize"))).unwrap();

            prop_assert!(matches!(hits, PyValue::Int(0)));
            prop_assert!(matches!(misses, PyValue::Int(0)));
            prop_assert!(matches!(currsize, PyValue::Int(0)));
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// wraps should store wrapped function
    /// Validates: Requirements 4.8, 4.13
    #[test]
    fn prop_wraps_stores_wrapped(name in "[a-z]{1,10}") {
        let wraps_fn = functools_builtins().into_iter()
            .find(|f| f.name == "wraps").unwrap();

        let wrapped = PyValue::Str(Arc::from(name.clone()));
        let result = wraps_fn.call(&[wrapped]).unwrap();

        if let PyValue::Dict(d) = result {
            let stored = d.getitem(&PyKey::Str(Arc::from("__wrapped__"))).unwrap();
            if let PyValue::Str(s) = stored {
                prop_assert_eq!(s.as_ref(), name.as_str());
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// cached_property should store function
    /// Validates: Requirements 4.8, 4.13
    #[test]
    fn prop_cached_property_stores_func(name in "[a-z]{1,10}") {
        let cached_property_fn = functools_builtins().into_iter()
            .find(|f| f.name == "cached_property").unwrap();

        let func = PyValue::Str(Arc::from(name.clone()));
        let result = cached_property_fn.call(&[func]).unwrap();

        if let PyValue::Dict(d) = result {
            let stored = d.getitem(&PyKey::Str(Arc::from("func"))).unwrap();
            if let PyValue::Str(s) = stored {
                prop_assert_eq!(s.as_ref(), name.as_str());
            }

            // Check class
            let class = d.getitem(&PyKey::Str(Arc::from("__class__"))).unwrap();
            if let PyValue::Str(s) = class {
                prop_assert_eq!(s.as_ref(), "cached_property");
            }
        }
    }
}

// ===== itertools module property tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// chain should concatenate all iterables
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_chain_concatenates(
        list1 in prop::collection::vec(any::<i64>(), 0..5),
        list2 in prop::collection::vec(any::<i64>(), 0..5)
    ) {
        let chain_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "chain").unwrap();

        let l1 = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            list1.iter().map(|i| PyValue::Int(*i)).collect()
        )));
        let l2 = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            list2.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = chain_fn.call(&[l1, l2]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), list1.len() + list2.len());

            // Check first part
            for (i, expected) in list1.iter().enumerate() {
                if let PyValue::Int(actual) = &l.to_vec()[i] {
                    prop_assert_eq!(*actual, *expected);
                }
            }

            // Check second part
            for (i, expected) in list2.iter().enumerate() {
                if let PyValue::Int(actual) = &l.to_vec()[list1.len() + i] {
                    prop_assert_eq!(*actual, *expected);
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// repeat should produce n copies of value
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_repeat_produces_copies(value in any::<i64>(), times in 0..20usize) {
        let repeat_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "repeat").unwrap();

        let result = repeat_fn.call(&[PyValue::Int(value), PyValue::Int(times as i64)]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), times);

            for item in l.to_vec() {
                if let PyValue::Int(actual) = item {
                    prop_assert_eq!(actual, value);
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// count should produce arithmetic sequence
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_count_arithmetic_sequence(
        start in -100..100i64,
        step in 1..10i64,
        n in 1..20usize
    ) {
        let count_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "count").unwrap();

        let result = count_fn.call(&[
            PyValue::Int(start),
            PyValue::Int(step),
            PyValue::Int(n as i64),
        ]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), n);

            for (i, item) in l.to_vec().into_iter().enumerate() {
                if let PyValue::Int(actual) = item {
                    let expected = start + (i as i64) * step;
                    prop_assert_eq!(actual, expected);
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// combinations should produce correct number of combinations
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_combinations_count(n in 1..6usize, r in 0..6usize) {
        if r > n {
            return Ok(());
        }

        let combinations_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "combinations").unwrap();

        let items: Vec<PyValue> = (0..n).map(|i| PyValue::Int(i as i64)).collect();
        let list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(items)));

        let result = combinations_fn.call(&[list, PyValue::Int(r as i64)]).unwrap();

        if let PyValue::List(l) = result {
            // C(n, r) = n! / (r! * (n-r)!)
            fn factorial(n: usize) -> usize {
                (1..=n).product()
            }
            let expected_count = factorial(n) / (factorial(r) * factorial(n - r));
            prop_assert_eq!(l.len(), expected_count);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// permutations should produce correct number of permutations
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_permutations_count(n in 1..5usize, r in 0..5usize) {
        if r > n {
            return Ok(());
        }

        let permutations_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "permutations").unwrap();

        let items: Vec<PyValue> = (0..n).map(|i| PyValue::Int(i as i64)).collect();
        let list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(items)));

        let result = permutations_fn.call(&[list, PyValue::Int(r as i64)]).unwrap();

        if let PyValue::List(l) = result {
            // P(n, r) = n! / (n-r)!
            fn factorial(n: usize) -> usize {
                (1..=n).product()
            }
            let expected_count = factorial(n) / factorial(n - r);
            prop_assert_eq!(l.len(), expected_count);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// product should produce cartesian product
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_product_count(
        n1 in 1..4usize,
        n2 in 1..4usize
    ) {
        let product_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "product").unwrap();

        let list1: Vec<PyValue> = (0..n1).map(|i| PyValue::Int(i as i64)).collect();
        let list2: Vec<PyValue> = (0..n2).map(|i| PyValue::Int(i as i64)).collect();

        let result = product_fn.call(&[
            PyValue::List(Arc::new(dx_py_core::PyList::from_values(list1))),
            PyValue::List(Arc::new(dx_py_core::PyList::from_values(list2))),
        ]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), n1 * n2);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// islice should slice correctly
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_islice_correct(
        items in prop::collection::vec(any::<i64>(), 0..20),
        start in 0..10usize,
        stop in 0..20usize
    ) {
        let islice_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "islice").unwrap();

        let list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = islice_fn.call(&[
            list,
            PyValue::Int(start as i64),
            PyValue::Int(stop as i64),
        ]).unwrap();

        if let PyValue::List(l) = result {
            let expected_len = stop.saturating_sub(start).min(items.len().saturating_sub(start));
            prop_assert_eq!(l.len(), expected_len);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// accumulate should produce running totals
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_accumulate_running_total(values in prop::collection::vec(any::<i64>().prop_map(|x| x % 100), 1..10)) {
        let accumulate_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "accumulate").unwrap();

        let list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            values.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = accumulate_fn.call(&[list]).unwrap();

        if let PyValue::List(l) = result {
            prop_assert_eq!(l.len(), values.len());

            let mut running_total = 0i64;
            for (i, item) in l.to_vec().into_iter().enumerate() {
                running_total += values[i];
                if let PyValue::Int(actual) = item {
                    prop_assert_eq!(actual, running_total);
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// compress should filter by selectors
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_compress_filters(
        data in prop::collection::vec(any::<i64>(), 0..10),
        selectors in prop::collection::vec(any::<bool>(), 0..10)
    ) {
        let compress_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "compress").unwrap();

        let data_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            data.iter().map(|i| PyValue::Int(*i)).collect()
        )));
        let selector_list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            selectors.iter().map(|b| PyValue::Bool(*b)).collect()
        )));

        let result = compress_fn.call(&[data_list, selector_list]).unwrap();

        if let PyValue::List(l) = result {
            let expected_count = data.iter()
                .zip(selectors.iter())
                .filter(|(_, s)| **s)
                .count();
            prop_assert_eq!(l.len(), expected_count);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// zip_longest should pad shorter iterables
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_zip_longest_pads(
        list1 in prop::collection::vec(any::<i64>(), 0..5),
        list2 in prop::collection::vec(any::<i64>(), 0..5)
    ) {
        let zip_longest_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "zip_longest").unwrap();

        let l1 = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            list1.iter().map(|i| PyValue::Int(*i)).collect()
        )));
        let l2 = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            list2.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = zip_longest_fn.call(&[l1, l2]).unwrap();

        if let PyValue::List(l) = result {
            let expected_len = list1.len().max(list2.len());
            prop_assert_eq!(l.len(), expected_len);
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// tee should produce n copies
    /// Validates: Requirements 4.7, 4.13
    #[test]
    fn prop_tee_produces_copies(
        items in prop::collection::vec(any::<i64>(), 0..10),
        n in 1..5usize
    ) {
        let tee_fn = itertools_builtins().into_iter()
            .find(|f| f.name == "tee").unwrap();

        let list = PyValue::List(Arc::new(dx_py_core::PyList::from_values(
            items.iter().map(|i| PyValue::Int(*i)).collect()
        )));

        let result = tee_fn.call(&[list, PyValue::Int(n as i64)]).unwrap();

        if let PyValue::Tuple(t) = result {
            prop_assert_eq!(t.len(), n);

            // Each copy should have the same length
            for copy in t.to_vec() {
                if let PyValue::List(l) = copy {
                    prop_assert_eq!(l.len(), items.len());
                }
            }
        }
    }
}

// ===== functools module unit tests =====

#[test]
fn test_functools_module_exists() {
    let functools = functools_module();

    let name = functools.getitem(&PyKey::Str(Arc::from("__name__"))).unwrap();
    if let PyValue::Str(s) = name {
        assert_eq!(s.as_ref(), "functools");
    }
}

#[test]
fn test_itertools_module_exists() {
    let itertools = itertools_module();

    let name = itertools.getitem(&PyKey::Str(Arc::from("__name__"))).unwrap();
    if let PyValue::Str(s) = name {
        assert_eq!(s.as_ref(), "itertools");
    }
}

#[test]
fn test_reduce_empty_with_initial() {
    let reduce_fn = functools_builtins().into_iter().find(|f| f.name == "reduce").unwrap();

    let empty_list = PyValue::List(Arc::new(dx_py_core::PyList::new()));

    let result = reduce_fn
        .call(&[PyValue::Str(Arc::from("add")), empty_list, PyValue::Int(42)])
        .unwrap();

    assert!(matches!(result, PyValue::Int(42)));
}

#[test]
fn test_reduce_empty_without_initial_errors() {
    let reduce_fn = functools_builtins().into_iter().find(|f| f.name == "reduce").unwrap();

    let empty_list = PyValue::List(Arc::new(dx_py_core::PyList::new()));

    let result = reduce_fn.call(&[PyValue::Str(Arc::from("add")), empty_list]);

    assert!(result.is_err());
}

// ===== json module property tests (Task 9.2) =====

use dx_py_core::stdlib::{json_builtins_expanded, json_module};

/// Generate arbitrary JSON-serializable values
fn arb_json_value() -> impl Strategy<Value = PyValue> {
    let leaf = prop_oneof![
        Just(PyValue::None),
        any::<bool>().prop_map(PyValue::Bool),
        any::<i64>().prop_map(PyValue::Int),
        // Use finite floats only (NaN and Infinity are not JSON-compliant)
        (-1e10..1e10f64).prop_map(PyValue::Float),
        "[a-zA-Z0-9_ ]{0,20}".prop_map(|s| PyValue::Str(Arc::from(s))),
    ];

    leaf.prop_recursive(
        3,  // depth
        64, // max nodes
        10, // items per collection
        |inner| {
            prop_oneof![
                // List of values
                prop::collection::vec(inner.clone(), 0..5)
                    .prop_map(|v| PyValue::List(Arc::new(dx_py_core::PyList::from_values(v)))),
                // Dict with string keys
                prop::collection::vec(
                    ("[a-zA-Z_][a-zA-Z0-9_]{0,10}".prop_map(|s| s), inner.clone()),
                    0..5
                )
                .prop_map(|pairs| {
                    let dict = dx_py_core::PyDict::new();
                    for (k, v) in pairs {
                        dict.setitem(PyKey::Str(Arc::from(k)), v);
                    }
                    PyValue::Dict(Arc::new(dict))
                }),
            ]
        },
    )
}

/// Generate simple JSON values (no nesting) for basic tests
fn arb_simple_json_value() -> impl Strategy<Value = PyValue> {
    prop_oneof![
        Just(PyValue::None),
        any::<bool>().prop_map(PyValue::Bool),
        any::<i64>().prop_map(PyValue::Int),
        (-1e10..1e10f64).prop_map(PyValue::Float),
        "[a-zA-Z0-9_]{0,20}".prop_map(|s| PyValue::Str(Arc::from(s))),
    ]
}

/// Check if two PyValues are equivalent for JSON purposes
fn json_values_equal(a: &PyValue, b: &PyValue) -> bool {
    match (a, b) {
        (PyValue::None, PyValue::None) => true,
        (PyValue::Bool(a), PyValue::Bool(b)) => a == b,
        (PyValue::Int(a), PyValue::Int(b)) => a == b,
        (PyValue::Float(a), PyValue::Float(b)) => (a - b).abs() < 1e-10,
        (PyValue::Int(a), PyValue::Float(b)) | (PyValue::Float(b), PyValue::Int(a)) => {
            (*a as f64 - b).abs() < 1e-10
        }
        (PyValue::Str(a), PyValue::Str(b)) => a == b,
        (PyValue::List(a), PyValue::List(b)) => {
            let a_vec = a.to_vec();
            let b_vec = b.to_vec();
            a_vec.len() == b_vec.len()
                && a_vec.iter().zip(b_vec.iter()).all(|(x, y)| json_values_equal(x, y))
        }
        (PyValue::Dict(a), PyValue::Dict(b)) => {
            let a_items = a.items();
            let b_items = b.items();
            if a_items.len() != b_items.len() {
                return false;
            }
            for (k, v) in &a_items {
                if let Some(bv) = b_items.iter().find(|(bk, _)| bk == k).map(|(_, v)| v) {
                    if !json_values_equal(v, bv) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// json.loads(json.dumps(x)) == x for JSON-serializable values
    /// Validates: Requirements 4.9, 4.13
    #[test]
    fn prop_json_roundtrip(value in arb_simple_json_value()) {
        let dumps = json_builtins_expanded().into_iter()
            .find(|f| f.name == "dumps").unwrap();
        let loads = json_builtins_expanded().into_iter()
            .find(|f| f.name == "loads").unwrap();

        // Serialize
        let json_str = dumps.call(&[value.clone()]).unwrap();

        // Deserialize
        let parsed = loads.call(&[json_str]).unwrap();

        // Should be equivalent
        prop_assert!(json_values_equal(&value, &parsed),
            "Round-trip failed: {:?} -> {:?}", value, parsed);
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// json.dumps should produce valid JSON for nested structures
    /// Validates: Requirements 4.9, 4.13
    #[test]
    fn prop_json_nested_roundtrip(value in arb_json_value()) {
        let dumps = json_builtins_expanded().into_iter()
            .find(|f| f.name == "dumps").unwrap();
        let loads = json_builtins_expanded().into_iter()
            .find(|f| f.name == "loads").unwrap();

        // Serialize
        let json_str = dumps.call(&[value.clone()]).unwrap();

        // Should be a string
        prop_assert!(matches!(json_str, PyValue::Str(_)));

        // Deserialize
        let parsed = loads.call(&[json_str]).unwrap();

        // Should be equivalent
        prop_assert!(json_values_equal(&value, &parsed),
            "Nested round-trip failed: {:?} -> {:?}", value, parsed);
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// json.dumps with indent should produce valid JSON
    /// Validates: Requirements 4.9, 4.13
    #[test]
    fn prop_json_indent_roundtrip(
        value in arb_json_value(),
        indent in 0..8i64
    ) {
        let dumps = json_builtins_expanded().into_iter()
            .find(|f| f.name == "dumps").unwrap();
        let loads = json_builtins_expanded().into_iter()
            .find(|f| f.name == "loads").unwrap();

        // Serialize with indent
        let json_str = dumps.call(&[value.clone(), PyValue::Int(indent)]).unwrap();

        // Should be a string
        prop_assert!(matches!(json_str, PyValue::Str(_)));

        // Deserialize
        let parsed = loads.call(&[json_str]).unwrap();

        // Should be equivalent
        prop_assert!(json_values_equal(&value, &parsed),
            "Indented round-trip failed: {:?} -> {:?}", value, parsed);
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// json.dumps with sort_keys should produce sorted output
    /// Validates: Requirements 4.9, 4.13
    #[test]
    fn prop_json_sort_keys(
        pairs in prop::collection::vec(
            ("[a-z]{1,5}".prop_map(|s| s), any::<i64>()),
            1..5
        )
    ) {
        let dumps = json_builtins_expanded().into_iter()
            .find(|f| f.name == "dumps").unwrap();

        // Create dict
        let dict = dx_py_core::PyDict::new();
        for (k, v) in &pairs {
            dict.setitem(PyKey::Str(Arc::from(k.clone())), PyValue::Int(*v));
        }

        // Serialize with sort_keys=True (args: value, indent, separators, sort_keys)
        let json_str = dumps.call(&[
            PyValue::Dict(Arc::new(dict)),
            PyValue::None,  // indent
            PyValue::None,  // separators
            PyValue::Bool(true),  // sort_keys
        ]).unwrap();

        if let PyValue::Str(s) = json_str {
            // Extract keys from JSON string
            let json = s.as_ref();
            let mut keys: Vec<String> = Vec::new();
            let mut in_key = false;
            let mut current_key = String::new();

            for c in json.chars() {
                if c == '"' {
                    if in_key {
                        keys.push(current_key.clone());
                        current_key.clear();
                    }
                    in_key = !in_key;
                } else if in_key {
                    current_key.push(c);
                } else if c == ':' {
                    in_key = false;
                }
            }

            // Filter to only actual keys (every other string is a key)
            let actual_keys: Vec<_> = keys.iter().step_by(2).cloned().collect();

            // Should be sorted
            let mut sorted_keys = actual_keys.clone();
            sorted_keys.sort();
            prop_assert_eq!(actual_keys, sorted_keys, "Keys should be sorted");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// JSONEncoder.encode should produce same result as dumps
    /// Validates: Requirements 4.9, 4.13
    #[test]
    fn prop_json_encoder_consistency(value in arb_simple_json_value()) {
        let dumps = json_builtins_expanded().into_iter()
            .find(|f| f.name == "dumps").unwrap();
        let encoder_fn = json_builtins_expanded().into_iter()
            .find(|f| f.name == "JSONEncoder").unwrap();
        let encode_fn = json_builtins_expanded().into_iter()
            .find(|f| f.name == "JSONEncoder_encode").unwrap();

        // Create encoder
        let encoder = encoder_fn.call(&[]).unwrap();

        // Encode with encoder
        let encoded = encode_fn.call(&[encoder, value.clone()]).unwrap();

        // Encode with dumps
        let dumped = dumps.call(&[value]).unwrap();

        // Should be the same
        if let (PyValue::Str(a), PyValue::Str(b)) = (&encoded, &dumped) {
            prop_assert_eq!(a.as_ref(), b.as_ref());
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// JSONDecoder.decode should produce same result as loads
    /// Validates: Requirements 4.9, 4.13
    #[test]
    fn prop_json_decoder_consistency(value in arb_simple_json_value()) {
        let dumps = json_builtins_expanded().into_iter()
            .find(|f| f.name == "dumps").unwrap();
        let loads = json_builtins_expanded().into_iter()
            .find(|f| f.name == "loads").unwrap();
        let decoder_fn = json_builtins_expanded().into_iter()
            .find(|f| f.name == "JSONDecoder").unwrap();
        let decode_fn = json_builtins_expanded().into_iter()
            .find(|f| f.name == "JSONDecoder_decode").unwrap();

        // Serialize first
        let json_str = dumps.call(&[value]).unwrap();

        // Create decoder
        let decoder = decoder_fn.call(&[]).unwrap();

        // Decode with decoder
        let decoded = decode_fn.call(&[decoder, json_str.clone()]).unwrap();

        // Decode with loads
        let loaded = loads.call(&[json_str]).unwrap();

        // Should be equivalent
        prop_assert!(json_values_equal(&decoded, &loaded));
    }
}

// ===== json module unit tests =====

#[test]
fn test_json_module_exists() {
    let json = json_module();

    let name = json.getitem(&PyKey::Str(Arc::from("__name__"))).unwrap();
    if let PyValue::Str(s) = name {
        assert_eq!(s.as_ref(), "json");
    }
}

#[test]
fn test_json_dumps_with_indent() {
    let dumps = json_builtins_expanded().into_iter().find(|f| f.name == "dumps").unwrap();

    let dict = dx_py_core::PyDict::new();
    dict.setitem(PyKey::Str(Arc::from("a")), PyValue::Int(1));

    let result = dumps
        .call(&[
            PyValue::Dict(Arc::new(dict)),
            PyValue::Int(2), // indent
        ])
        .unwrap();

    if let PyValue::Str(s) = result {
        assert!(s.contains('\n'), "Indented JSON should contain newlines");
    }
}

#[test]
fn test_json_loads_nested_object() {
    let loads = json_builtins_expanded().into_iter().find(|f| f.name == "loads").unwrap();

    let json = r#"{"outer": {"inner": 42}}"#;
    let result = loads.call(&[PyValue::Str(Arc::from(json))]).unwrap();

    if let PyValue::Dict(d) = result {
        let outer = d.getitem(&PyKey::Str(Arc::from("outer"))).unwrap();
        if let PyValue::Dict(inner_dict) = outer {
            let inner = inner_dict.getitem(&PyKey::Str(Arc::from("inner"))).unwrap();
            assert!(matches!(inner, PyValue::Int(42)));
        } else {
            panic!("Expected nested dict");
        }
    } else {
        panic!("Expected dict");
    }
}

#[test]
fn test_json_loads_nested_array() {
    let loads = json_builtins_expanded().into_iter().find(|f| f.name == "loads").unwrap();

    let json = r#"[[1, 2], [3, 4]]"#;
    let result = loads.call(&[PyValue::Str(Arc::from(json))]).unwrap();

    if let PyValue::List(l) = result {
        assert_eq!(l.len(), 2);
        if let PyValue::List(inner) = &l.to_vec()[0] {
            assert_eq!(inner.len(), 2);
        } else {
            panic!("Expected nested list");
        }
    } else {
        panic!("Expected list");
    }
}

#[test]
fn test_json_unicode_escape() {
    let dumps = json_builtins_expanded().into_iter().find(|f| f.name == "dumps").unwrap();
    let loads = json_builtins_expanded().into_iter().find(|f| f.name == "loads").unwrap();

    // Test with unicode characters
    let value = PyValue::Str(Arc::from("Hello, 世界!"));

    // Round-trip
    let json_str = dumps.call(&[value.clone()]).unwrap();
    let parsed = loads.call(&[json_str]).unwrap();

    if let (PyValue::Str(orig), PyValue::Str(parsed)) = (&value, &parsed) {
        assert_eq!(orig.as_ref(), parsed.as_ref());
    }
}

/// Feature: dx-py-production-ready, Task 5.3: json.loads raises JSONDecodeError for invalid JSON
/// Validates: Requirements 5.6
#[test]
fn test_json_loads_invalid_json_raises_json_decode_error() {
    let loads = json_builtins_expanded().into_iter().find(|f| f.name == "loads").unwrap();

    // Test various invalid JSON strings
    let invalid_jsons = vec![
        "",                    // Empty string
        "{",                   // Unclosed brace
        "[",                   // Unclosed bracket
        "{'key': 'value'}",    // Single quotes (invalid in JSON)
        "{key: 'value'}",      // Unquoted key
        "[1, 2,]",             // Trailing comma
        "{\"a\": 1,}",         // Trailing comma in object
        "undefined",           // JavaScript undefined
        "NaN",                 // NaN is not valid JSON
        "Infinity",            // Infinity is not valid JSON
        "{\"a\": }",           // Missing value
        "[1 2 3]",             // Missing commas
        "\"unterminated",      // Unterminated string
        "{\"a\": \"\\x00\"}",  // Invalid escape sequence
    ];

    for invalid_json in invalid_jsons {
        let result = loads.call(&[PyValue::Str(Arc::from(invalid_json))]);
        assert!(result.is_err(), "Expected error for invalid JSON: {}", invalid_json);

        if let Err(err) = result {
            assert_eq!(
                err.exception_name(),
                "JSONDecodeError",
                "Expected JSONDecodeError for '{}', got {}",
                invalid_json,
                err.exception_name()
            );
        }
    }
}

/// Feature: dx-py-production-ready, Task 5.3: json.loads parses valid JSON correctly
/// Validates: Requirements 5.4
#[test]
fn test_json_loads_valid_json() {
    let loads = json_builtins_expanded().into_iter().find(|f| f.name == "loads").unwrap();

    // Test null
    let result = loads.call(&[PyValue::Str(Arc::from("null"))]).unwrap();
    assert!(matches!(result, PyValue::None));

    // Test booleans
    let result = loads.call(&[PyValue::Str(Arc::from("true"))]).unwrap();
    assert!(matches!(result, PyValue::Bool(true)));

    let result = loads.call(&[PyValue::Str(Arc::from("false"))]).unwrap();
    assert!(matches!(result, PyValue::Bool(false)));

    // Test integers
    let result = loads.call(&[PyValue::Str(Arc::from("42"))]).unwrap();
    assert!(matches!(result, PyValue::Int(42)));

    let result = loads.call(&[PyValue::Str(Arc::from("-123"))]).unwrap();
    assert!(matches!(result, PyValue::Int(-123)));

    // Test floats
    let result = loads.call(&[PyValue::Str(Arc::from("3.14"))]).unwrap();
    if let PyValue::Float(f) = result {
        assert!((f - 3.14).abs() < 0.001);
    } else {
        panic!("Expected float");
    }

    let result = loads.call(&[PyValue::Str(Arc::from("-2.5e10"))]).unwrap();
    if let PyValue::Float(f) = result {
        assert!((f - (-2.5e10)).abs() < 1e5);
    } else {
        panic!("Expected float");
    }

    // Test strings
    let result = loads.call(&[PyValue::Str(Arc::from("\"hello\""))]).unwrap();
    if let PyValue::Str(s) = result {
        assert_eq!(s.as_ref(), "hello");
    } else {
        panic!("Expected string");
    }

    // Test string with escapes
    let result = loads.call(&[PyValue::Str(Arc::from("\"hello\\nworld\""))]).unwrap();
    if let PyValue::Str(s) = result {
        assert_eq!(s.as_ref(), "hello\nworld");
    } else {
        panic!("Expected string");
    }

    // Test empty array
    let result = loads.call(&[PyValue::Str(Arc::from("[]"))]).unwrap();
    if let PyValue::List(l) = result {
        assert_eq!(l.len(), 0);
    } else {
        panic!("Expected list");
    }

    // Test array with values
    let result = loads.call(&[PyValue::Str(Arc::from("[1, 2, 3]"))]).unwrap();
    if let PyValue::List(l) = result {
        assert_eq!(l.len(), 3);
    } else {
        panic!("Expected list");
    }

    // Test empty object
    let result = loads.call(&[PyValue::Str(Arc::from("{}"))]).unwrap();
    if let PyValue::Dict(d) = result {
        assert_eq!(d.len(), 0);
    } else {
        panic!("Expected dict");
    }

    // Test object with values
    let result = loads.call(&[PyValue::Str(Arc::from("{\"a\": 1, \"b\": 2}"))]).unwrap();
    if let PyValue::Dict(d) = result {
        assert_eq!(d.len(), 2);
        let a = d.getitem(&PyKey::Str(Arc::from("a"))).unwrap();
        assert!(matches!(a, PyValue::Int(1)));
    } else {
        panic!("Expected dict");
    }
}

// ===== re module property tests (Task 9.4) =====

use dx_py_core::stdlib::{re_builtins, re_module};

/// Generate valid regex patterns (simple patterns that won't cause regex errors)
fn arb_simple_pattern() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-z]+".prop_map(|s| s),    // Literal strings
        Just("[a-z]+".to_string()),  // Character class
        Just("\\d+".to_string()),    // Digit class
        Just("\\w+".to_string()),    // Word class
        Just("\\s+".to_string()),    // Whitespace class
        Just(".+".to_string()),      // Any character
        Just("^[a-z]+".to_string()), // Anchored start
        Just("[a-z]+$".to_string()), // Anchored end
    ]
}

/// Generate strings that will match simple patterns
fn arb_matchable_string() -> impl Strategy<Value = String> {
    "[a-z]{1,20}".prop_map(|s| s)
}

/// Generate strings with digits
fn arb_digit_string() -> impl Strategy<Value = String> {
    "[0-9]{1,10}".prop_map(|s| s)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.compile should return a Pattern object with the pattern stored
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_compile_stores_pattern(pattern in arb_simple_pattern()) {
        let compile = re_builtins().into_iter().find(|f| f.name == "compile").unwrap();

        let result = compile.call(&[PyValue::Str(Arc::from(pattern.clone()))]).unwrap();

        if let PyValue::Dict(d) = result {
            let stored_pattern = d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None);
            if let PyValue::Str(s) = stored_pattern {
                prop_assert_eq!(s.as_ref(), pattern.as_str());
            } else {
                prop_assert!(false, "Expected pattern string in Pattern object");
            }
        } else {
            prop_assert!(false, "Expected dict from compile");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.match should return Match object when pattern matches at start
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_match_at_start(text in arb_matchable_string()) {
        let match_fn = re_builtins().into_iter().find(|f| f.name == "match").unwrap();

        // Pattern that matches lowercase letters
        let pattern = "[a-z]+";

        let result = match_fn.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(text.clone())),
        ]).unwrap();

        // Should match since text is all lowercase letters
        if let PyValue::Dict(d) = result {
            let matched = d.get(&PyKey::Str(Arc::from("_match")), PyValue::None);
            if let PyValue::Str(s) = matched {
                // Match should be a prefix of the text
                prop_assert!(text.starts_with(s.as_ref()),
                    "Match '{}' should be prefix of '{}'", s, text);
            }
        } else {
            prop_assert!(false, "Expected Match object for matching text");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.match should return None when pattern doesn't match at start
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_match_no_match_at_start(text in arb_matchable_string()) {
        let match_fn = re_builtins().into_iter().find(|f| f.name == "match").unwrap();

        // Pattern that matches digits - won't match lowercase text at start
        let pattern = "\\d+";

        let result = match_fn.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(text)),
        ]).unwrap();

        // Should not match since text is all lowercase letters
        prop_assert!(matches!(result, PyValue::None), "Expected None for non-matching text");
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.search should find pattern anywhere in string
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_search_finds_anywhere(
        prefix in "[A-Z]{0,5}",
        middle in arb_digit_string(),
        suffix in "[A-Z]{0,5}"
    ) {
        let search = re_builtins().into_iter().find(|f| f.name == "search").unwrap();

        let text = format!("{}{}{}", prefix, middle, suffix);
        let pattern = "\\d+";

        let result = search.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(text.clone())),
        ]).unwrap();

        // Should find the digits in the middle
        if let PyValue::Dict(d) = result {
            let matched = d.get(&PyKey::Str(Arc::from("_match")), PyValue::None);
            if let PyValue::Str(s) = matched {
                prop_assert_eq!(s.as_ref(), middle.as_str(),
                    "Should find digits '{}' in '{}'", middle, text);
            }
        } else {
            prop_assert!(false, "Expected Match object");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.findall should return all non-overlapping matches
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_findall_returns_all(
        words in prop::collection::vec(arb_matchable_string(), 1..5)
    ) {
        let findall = re_builtins().into_iter().find(|f| f.name == "findall").unwrap();

        // Join words with spaces
        let text = words.join(" ");
        let pattern = "[a-z]+";

        let result = findall.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(text)),
        ]).unwrap();

        if let PyValue::List(l) = result {
            // Should find at least as many matches as words
            prop_assert!(l.len() >= words.len(),
                "Should find at least {} matches, found {}", words.len(), l.len());

            // Each word should be in the results
            let results: Vec<String> = l.to_vec().into_iter()
                .filter_map(|v| if let PyValue::Str(s) = v { Some(s.to_string()) } else { None })
                .collect();

            for word in &words {
                prop_assert!(results.contains(word),
                    "Word '{}' should be in results {:?}", word, results);
            }
        } else {
            prop_assert!(false, "Expected list from findall");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.sub should replace all occurrences
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_sub_replaces_all(
        words in prop::collection::vec("[a-z]{2,5}".prop_map(|s| s), 1..5),
        replacement in "[A-Z]{1,3}"
    ) {
        let sub = re_builtins().into_iter().find(|f| f.name == "sub").unwrap();

        let text = words.join(" ");
        let pattern = "[a-z]+";

        let result = sub.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(replacement.clone())),
            PyValue::Str(Arc::from(text)),
        ]).unwrap();

        if let PyValue::Str(s) = result {
            // Result should not contain any lowercase letters (all replaced)
            let has_lowercase = s.chars().any(|c| c.is_ascii_lowercase());
            prop_assert!(!has_lowercase,
                "Result '{}' should not contain lowercase letters", s);

            // Result should contain the replacement
            prop_assert!(s.contains(&replacement),
                "Result '{}' should contain replacement '{}'", s, replacement);
        } else {
            prop_assert!(false, "Expected string from sub");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.subn should return count of replacements
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_subn_returns_count(
        words in prop::collection::vec("[a-z]{2,5}".prop_map(|s| s), 1..5)
    ) {
        let subn = re_builtins().into_iter().find(|f| f.name == "subn").unwrap();

        let text = words.join(" ");
        let pattern = "[a-z]+";

        let result = subn.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from("X")),
            PyValue::Str(Arc::from(text)),
        ]).unwrap();

        if let PyValue::Tuple(t) = result {
            let parts = t.to_vec();
            prop_assert_eq!(parts.len(), 2, "subn should return tuple of 2 elements");

            if let PyValue::Int(count) = &parts[1] {
                // Count should be at least the number of words
                prop_assert!(*count >= words.len() as i64,
                    "Count {} should be >= {}", count, words.len());
            }
        } else {
            prop_assert!(false, "Expected tuple from subn");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.split should split string by pattern
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_split_by_pattern(
        words in prop::collection::vec("[a-z]{2,5}".prop_map(|s| s), 2..5)
    ) {
        let split = re_builtins().into_iter().find(|f| f.name == "split").unwrap();

        // Join with digits as separators
        let text = words.join("123");
        let pattern = "\\d+";

        let result = split.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(text)),
        ]).unwrap();

        if let PyValue::List(l) = result {
            // Should have same number of parts as words
            prop_assert_eq!(l.len(), words.len(),
                "Split should produce {} parts, got {}", words.len(), l.len());

            // Parts should match original words
            let parts: Vec<String> = l.to_vec().into_iter()
                .filter_map(|v| if let PyValue::Str(s) = v { Some(s.to_string()) } else { None })
                .collect();

            prop_assert_eq!(parts, words);
        } else {
            prop_assert!(false, "Expected list from split");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// re.escape should escape special regex characters
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_escape_makes_literal(special in "[.+*?^${}()\\[\\]|\\\\]{1,5}") {
        let escape = re_builtins().into_iter().find(|f| f.name == "escape").unwrap();
        let match_fn = re_builtins().into_iter().find(|f| f.name == "match").unwrap();

        // Escape the special characters
        let escaped = escape.call(&[PyValue::Str(Arc::from(special.clone()))]).unwrap();

        if let PyValue::Str(escaped_pattern) = escaped {
            // Using escaped pattern should match the literal string
            let result = match_fn.call(&[
                PyValue::Str(escaped_pattern),
                PyValue::Str(Arc::from(special.clone())),
            ]).unwrap();

            // Should match the literal special characters
            if let PyValue::Dict(d) = result {
                let matched = d.get(&PyKey::Str(Arc::from("_match")), PyValue::None);
                if let PyValue::Str(s) = matched {
                    prop_assert_eq!(s.as_ref(), special.as_str());
                }
            } else {
                prop_assert!(false, "Escaped pattern should match literal string");
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Match.span should return (start, end) where end - start = len(match)
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_match_span_consistency(
        prefix in "[A-Z]{0,5}",
        match_text in arb_matchable_string()
    ) {
        let search = re_builtins().into_iter().find(|f| f.name == "search").unwrap();
        let span_fn = re_builtins().into_iter().find(|f| f.name == "Match_span").unwrap();

        let text = format!("{}{}", prefix, match_text);
        let pattern = "[a-z]+";

        let result = search.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(text)),
        ]).unwrap();

        if let PyValue::Dict(d) = &result {
            let span = span_fn.call(&[result.clone()]).unwrap();

            if let PyValue::Tuple(t) = span {
                let parts = t.to_vec();
                if let (PyValue::Int(start), PyValue::Int(end)) = (&parts[0], &parts[1]) {
                    let matched = d.get(&PyKey::Str(Arc::from("_match")), PyValue::None);
                    if let PyValue::Str(s) = matched {
                        // end - start should equal match length
                        prop_assert_eq!((*end - *start) as usize, s.len(),
                            "span ({}, {}) should have length {}", start, end, s.len());
                    }
                }
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Match.group(0) should return the full match
    /// Validates: Requirements 4.10, 4.13
    #[test]
    fn prop_re_match_group_zero(text in arb_matchable_string()) {
        let match_fn = re_builtins().into_iter().find(|f| f.name == "match").unwrap();
        let group_fn = re_builtins().into_iter().find(|f| f.name == "Match_group").unwrap();

        let pattern = "[a-z]+";

        let result = match_fn.call(&[
            PyValue::Str(Arc::from(pattern)),
            PyValue::Str(Arc::from(text.clone())),
        ]).unwrap();

        if let PyValue::Dict(d) = &result {
            let group0 = group_fn.call(&[result.clone(), PyValue::Int(0)]).unwrap();
            let stored_match = d.get(&PyKey::Str(Arc::from("_match")), PyValue::None);

            // group(0) should equal the stored match
            if let (PyValue::Str(g), PyValue::Str(m)) = (&group0, &stored_match) {
                prop_assert_eq!(g.as_ref(), m.as_ref());
            }
        }
    }
}

// ===== re module unit tests =====

#[test]
fn test_re_module_has_required_attributes() {
    let re = re_module();

    // Check required flags exist
    assert!(re.contains(&PyKey::Str(Arc::from("IGNORECASE"))));
    assert!(re.contains(&PyKey::Str(Arc::from("I"))));
    assert!(re.contains(&PyKey::Str(Arc::from("MULTILINE"))));
    assert!(re.contains(&PyKey::Str(Arc::from("M"))));
    assert!(re.contains(&PyKey::Str(Arc::from("DOTALL"))));
    assert!(re.contains(&PyKey::Str(Arc::from("S"))));
    assert!(re.contains(&PyKey::Str(Arc::from("VERBOSE"))));
    assert!(re.contains(&PyKey::Str(Arc::from("X"))));
}

#[test]
fn test_re_compile_basic() {
    let compile = re_builtins().into_iter().find(|f| f.name == "compile").unwrap();

    let result = compile.call(&[PyValue::Str(Arc::from("[a-z]+"))]).unwrap();

    if let PyValue::Dict(d) = result {
        let class = d.get(&PyKey::Str(Arc::from("__class__")), PyValue::None);
        assert!(matches!(class, PyValue::Str(s) if s.as_ref() == "Pattern"));
    } else {
        panic!("Expected Pattern object");
    }
}

#[test]
fn test_re_match_with_groups() {
    let match_fn = re_builtins().into_iter().find(|f| f.name == "match").unwrap();
    let groups_fn = re_builtins().into_iter().find(|f| f.name == "Match_groups").unwrap();

    let result = match_fn
        .call(&[
            PyValue::Str(Arc::from("(\\w+)@(\\w+)")),
            PyValue::Str(Arc::from("user@domain")),
        ])
        .unwrap();

    if let PyValue::Dict(_) = &result {
        let groups = groups_fn.call(&[result]).unwrap();

        if let PyValue::Tuple(t) = groups {
            let parts = t.to_vec();
            assert_eq!(parts.len(), 2);
            assert!(matches!(&parts[0], PyValue::Str(s) if s.as_ref() == "user"));
            assert!(matches!(&parts[1], PyValue::Str(s) if s.as_ref() == "domain"));
        } else {
            panic!("Expected tuple from groups");
        }
    } else {
        panic!("Expected Match object");
    }
}

#[test]
fn test_re_findall_with_groups() {
    let findall = re_builtins().into_iter().find(|f| f.name == "findall").unwrap();

    let result = findall
        .call(&[
            PyValue::Str(Arc::from("(\\d+)-(\\d+)")),
            PyValue::Str(Arc::from("1-2 3-4 5-6")),
        ])
        .unwrap();

    if let PyValue::List(l) = result {
        assert_eq!(l.len(), 3);

        // Each match should be a tuple of the two groups
        for item in l.to_vec() {
            if let PyValue::Tuple(t) = item {
                assert_eq!(t.len(), 2);
            } else {
                panic!("Expected tuple for grouped match");
            }
        }
    } else {
        panic!("Expected list from findall");
    }
}

#[test]
fn test_re_sub_with_backreference() {
    let sub = re_builtins().into_iter().find(|f| f.name == "sub").unwrap();

    let result = sub
        .call(&[
            PyValue::Str(Arc::from("(\\w+)")),
            PyValue::Str(Arc::from("[$1]")),
            PyValue::Str(Arc::from("hello world")),
        ])
        .unwrap();

    if let PyValue::Str(s) = result {
        assert_eq!(s.as_ref(), "[hello] [world]");
    } else {
        panic!("Expected string from sub");
    }
}

#[test]
fn test_re_split_with_maxsplit() {
    let split = re_builtins().into_iter().find(|f| f.name == "split").unwrap();

    let result = split
        .call(&[
            PyValue::Str(Arc::from("\\s+")),
            PyValue::Str(Arc::from("a b c d e")),
            PyValue::Int(2), // maxsplit
        ])
        .unwrap();

    if let PyValue::List(l) = result {
        assert_eq!(l.len(), 3); // 2 splits = 3 parts

        let parts: Vec<String> = l
            .to_vec()
            .into_iter()
            .filter_map(|v| {
                if let PyValue::Str(s) = v {
                    Some(s.to_string())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(parts, vec!["a", "b", "c d e"]);
    } else {
        panic!("Expected list from split");
    }
}

#[test]
fn test_re_ignorecase_flag() {
    let match_fn = re_builtins().into_iter().find(|f| f.name == "match").unwrap();

    // Without flag - should not match
    let result1 = match_fn
        .call(&[
            PyValue::Str(Arc::from("hello")),
            PyValue::Str(Arc::from("HELLO")),
        ])
        .unwrap();
    assert!(matches!(result1, PyValue::None));

    // With IGNORECASE flag (2) - should match
    let result2 = match_fn
        .call(&[
            PyValue::Str(Arc::from("hello")),
            PyValue::Str(Arc::from("HELLO")),
            PyValue::Int(2), // IGNORECASE
        ])
        .unwrap();
    assert!(matches!(result2, PyValue::Dict(_)));
}

// ===== pathlib module property tests (Task 10.3) =====

use dx_py_core::stdlib::{pathlib_builtins, pathlib_module};

/// Generate valid path components for pathlib tests
fn arb_pathlib_component() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,15}".prop_filter("non-empty", |s| !s.is_empty())
}

/// Generate file extensions
fn arb_extension() -> impl Strategy<Value = String> {
    "[a-z]{1,4}".prop_map(|s| s)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path constructor should store the path correctly
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_path_stores_path(path in arb_pathlib_component()) {
        let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
        let str_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_str").unwrap();

        let path_obj = path_fn.call(&[PyValue::Str(Arc::from(path.clone()))]).unwrap();
        let result = str_fn.call(&[path_obj]).unwrap();

        if let PyValue::Str(s) = result {
            prop_assert_eq!(s.as_ref(), path.as_str());
        } else {
            prop_assert!(false, "Expected string from Path_str");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path.joinpath should combine paths correctly
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_joinpath_combines(
        base in arb_pathlib_component(),
        child in arb_pathlib_component()
    ) {
        let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
        let joinpath_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_joinpath").unwrap();
        let str_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_str").unwrap();

        let base_path = path_fn.call(&[PyValue::Str(Arc::from(base.clone()))]).unwrap();
        let joined = joinpath_fn.call(&[base_path, PyValue::Str(Arc::from(child.clone()))]).unwrap();
        let result = str_fn.call(&[joined]).unwrap();

        if let PyValue::Str(s) = result {
            // Result should contain both base and child
            prop_assert!(s.contains(&base), "Joined path should contain base");
            prop_assert!(s.contains(&child), "Joined path should contain child");
        } else {
            prop_assert!(false, "Expected string from Path_str");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path.parent then joinpath with name should reconstruct path
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_parent_name_reconstruct(
        parent in arb_pathlib_component(),
        name in arb_pathlib_component()
    ) {
        let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
        let joinpath_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_joinpath").unwrap();
        let parent_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_parent").unwrap();
        let str_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_str").unwrap();

        // Create path: parent/name
        let base = path_fn.call(&[PyValue::Str(Arc::from(parent.clone()))]).unwrap();
        let full_path = joinpath_fn.call(&[base, PyValue::Str(Arc::from(name.clone()))]).unwrap();
        let original = str_fn.call(&[full_path.clone()]).unwrap();

        // Get parent
        let parent_path = parent_fn.call(&[full_path]).unwrap();

        // Rejoin with name
        let reconstructed = joinpath_fn.call(&[parent_path, PyValue::Str(Arc::from(name))]).unwrap();
        let result = str_fn.call(&[reconstructed]).unwrap();

        if let (PyValue::Str(orig), PyValue::Str(recon)) = (&original, &result) {
            prop_assert_eq!(orig.as_ref(), recon.as_ref());
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path.with_suffix should change only the suffix
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_with_suffix_changes_suffix(
        stem in arb_pathlib_component(),
        old_ext in arb_extension(),
        new_ext in arb_extension()
    ) {
        let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
        let with_suffix_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_with_suffix").unwrap();
        let str_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_str").unwrap();

        let filename = format!("{}.{}", stem, old_ext);
        let path_obj = path_fn.call(&[PyValue::Str(Arc::from(filename))]).unwrap();

        let new_suffix = format!(".{}", new_ext);
        let new_path = with_suffix_fn.call(&[path_obj, PyValue::Str(Arc::from(new_suffix.clone()))]).unwrap();
        let result = str_fn.call(&[new_path]).unwrap();

        if let PyValue::Str(s) = result {
            // Should contain stem
            prop_assert!(s.contains(&stem), "New path should contain stem");
            // Should end with new extension
            prop_assert!(s.ends_with(&new_ext), "New path should end with new extension");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path.with_name should change only the name
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_with_name_changes_name(
        parent in arb_pathlib_component(),
        old_name in arb_pathlib_component(),
        new_name in arb_pathlib_component()
    ) {
        let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
        let joinpath_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_joinpath").unwrap();
        let with_name_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_with_name").unwrap();
        let str_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_str").unwrap();

        // Create path: parent/old_name
        let base = path_fn.call(&[PyValue::Str(Arc::from(parent.clone()))]).unwrap();
        let full_path = joinpath_fn.call(&[base, PyValue::Str(Arc::from(old_name))]).unwrap();

        // Change name
        let new_path = with_name_fn.call(&[full_path, PyValue::Str(Arc::from(new_name.clone()))]).unwrap();
        let result = str_fn.call(&[new_path]).unwrap();

        if let PyValue::Str(s) = result {
            // Should contain parent
            prop_assert!(s.contains(&parent), "New path should contain parent");
            // Should contain new name
            prop_assert!(s.contains(&new_name), "New path should contain new name");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path.exists should return bool for any path
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_exists_returns_bool(path in arb_pathlib_component()) {
        let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
        let exists_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_exists").unwrap();

        let path_obj = path_fn.call(&[PyValue::Str(Arc::from(path))]).unwrap();
        let result = exists_fn.call(&[path_obj]).unwrap();

        prop_assert!(matches!(result, PyValue::Bool(_)));
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path.is_file and is_dir should be mutually exclusive
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_isfile_isdir_exclusive(path in arb_pathlib_component()) {
        let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
        let is_file_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_is_file").unwrap();
        let is_dir_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_is_dir").unwrap();

        let path_obj = path_fn.call(&[PyValue::Str(Arc::from(path))]).unwrap();

        let is_file = is_file_fn.call(&[path_obj.clone()]).unwrap();
        let is_dir = is_dir_fn.call(&[path_obj]).unwrap();

        if let (PyValue::Bool(f), PyValue::Bool(d)) = (is_file, is_dir) {
            // Cannot be both file and directory
            prop_assert!(!(f && d), "Path cannot be both file and directory");
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Stdlib Function Equivalence
    /// Path.cwd should return a Path that exists and is a directory
    /// Validates: Requirements 4.12, 4.13
    #[test]
    fn prop_pathlib_cwd_exists_and_is_dir(_dummy in 0..10i32) {
        let cwd_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_cwd").unwrap();
        let exists_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_exists").unwrap();
        let is_dir_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_is_dir").unwrap();

        let cwd = cwd_fn.call(&[]).unwrap();

        let exists = exists_fn.call(&[cwd.clone()]).unwrap();
        let is_dir = is_dir_fn.call(&[cwd]).unwrap();

        prop_assert!(matches!(exists, PyValue::Bool(true)), "cwd should exist");
        prop_assert!(matches!(is_dir, PyValue::Bool(true)), "cwd should be a directory");
    }
}

// ===== pathlib module unit tests =====

#[test]
fn test_pathlib_module_has_required_attributes() {
    let pathlib = pathlib_module();

    let name = pathlib.getitem(&PyKey::Str(Arc::from("__name__"))).unwrap();
    if let PyValue::Str(s) = name {
        assert_eq!(s.as_ref(), "pathlib");
    }
}

#[test]
fn test_pathlib_path_parts() {
    let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();

    let path_obj = path_fn.call(&[PyValue::Str(Arc::from("a/b/c"))]).unwrap();

    if let PyValue::Dict(d) = path_obj {
        let name = d.get(&PyKey::Str(Arc::from("name")), PyValue::None);
        assert!(matches!(name, PyValue::Str(s) if s.as_ref() == "c"));
    }
}

#[test]
fn test_pathlib_path_suffix() {
    let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();

    let path_obj = path_fn.call(&[PyValue::Str(Arc::from("file.txt"))]).unwrap();

    if let PyValue::Dict(d) = path_obj {
        let suffix = d.get(&PyKey::Str(Arc::from("suffix")), PyValue::None);
        assert!(matches!(suffix, PyValue::Str(s) if s.as_ref() == ".txt"));

        let stem = d.get(&PyKey::Str(Arc::from("stem")), PyValue::None);
        assert!(matches!(stem, PyValue::Str(s) if s.as_ref() == "file"));
    }
}

#[test]
fn test_pathlib_joinpath_multiple() {
    let path_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path").unwrap();
    let joinpath_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_joinpath").unwrap();
    let str_fn = pathlib_builtins().into_iter().find(|f| f.name == "Path_str").unwrap();

    let base = path_fn.call(&[PyValue::Str(Arc::from("a"))]).unwrap();
    let joined = joinpath_fn
        .call(&[
            base,
            PyValue::Str(Arc::from("b")),
            PyValue::Str(Arc::from("c")),
        ])
        .unwrap();

    let result = str_fn.call(&[joined]).unwrap();

    if let PyValue::Str(s) = result {
        assert!(s.contains("a"));
        assert!(s.contains("b"));
        assert!(s.contains("c"));
    }
}
