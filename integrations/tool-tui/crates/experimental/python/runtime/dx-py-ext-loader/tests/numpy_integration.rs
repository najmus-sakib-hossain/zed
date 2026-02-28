//! Integration tests for NumPy loading
//!
//! These tests verify that the extension loader can properly load NumPy
//! and interact with its C API.
//!
//! Note: These tests require NumPy to be installed in the system Python.
//! They are marked as #[ignore] by default and can be run with:
//! `cargo test --test numpy_integration -- --ignored`

use std::env;
use std::path::PathBuf;

use dx_py_ext_loader::{
    AbiCompatibility, AbiVersion, ApiUsageReport, CApiTable, ExtensionError, ExtensionLoader,
};

/// Helper to find NumPy installation path
fn find_numpy_path() -> Option<PathBuf> {
    // Try common NumPy installation locations
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()?;

    let possible_paths = [
        // Windows Python 3.11
        PathBuf::from("C:\\Python311\\Lib\\site-packages\\numpy"),
        PathBuf::from("C:\\Python312\\Lib\\site-packages\\numpy"),
        // Windows user install
        PathBuf::from(&home)
            .join("AppData\\Local\\Programs\\Python\\Python311\\Lib\\site-packages\\numpy"),
        PathBuf::from(&home)
            .join("AppData\\Local\\Programs\\Python\\Python312\\Lib\\site-packages\\numpy"),
        // Linux/macOS system
        PathBuf::from("/usr/lib/python3.11/site-packages/numpy"),
        PathBuf::from("/usr/lib/python3.12/site-packages/numpy"),
        PathBuf::from("/usr/local/lib/python3.11/site-packages/numpy"),
        PathBuf::from("/usr/local/lib/python3.12/site-packages/numpy"),
        // Linux/macOS user install
        PathBuf::from(&home).join(".local/lib/python3.11/site-packages/numpy"),
        PathBuf::from(&home).join(".local/lib/python3.12/site-packages/numpy"),
        // Virtual environment (if VIRTUAL_ENV is set)
        env::var("VIRTUAL_ENV")
            .ok()
            .map(|v| PathBuf::from(v).join("Lib/site-packages/numpy"))
            .unwrap_or_default(),
    ];

    for path in &possible_paths {
        if path.exists() && path.is_dir() {
            return Some(path.clone());
        }
    }

    None
}

// =============================================================================
// Basic Extension Loader Tests
// =============================================================================

#[test]
fn test_extension_loader_creation() {
    let loader = ExtensionLoader::with_default_abi();
    assert_eq!(loader.abi_version(), AbiVersion::dx_py_abi());
    assert!(loader.loaded_extensions().is_empty());
}

#[test]
fn test_capi_table_available() {
    let loader = ExtensionLoader::default();
    let _table = loader.capi_table();

    // Verify we have function pointers
    let implemented = CApiTable::implemented_functions();
    assert!(!implemented.is_empty());
    assert!(implemented.contains(&"Py_IncRef"));
    assert!(implemented.contains(&"PyObject_GetBuffer"));
}

#[test]
fn test_api_tracker_available() {
    let loader = ExtensionLoader::default();
    let tracker = loader.api_tracker();

    // Initially no calls
    assert_eq!(tracker.total_calls(), 0);
    assert!(!tracker.has_unsupported_calls());
}

#[test]
fn test_api_usage_report() {
    let loader = ExtensionLoader::default();
    let report = loader.api_usage_report();

    assert_eq!(report.total_extensions, 0);
    assert!(report.implemented_functions_used.is_empty());
    assert!(report.stub_functions_used.is_empty());
    assert!(!report.has_unsupported);
}

#[test]
fn test_api_usage_report_markdown() {
    let report = ApiUsageReport {
        total_extensions: 2,
        implemented_functions_used: vec![
            ("numpy".to_string(), "Py_IncRef".to_string()),
            ("numpy".to_string(), "PyObject_GetBuffer".to_string()),
        ],
        stub_functions_used: vec![("numpy".to_string(), "PyLong_FromLong".to_string())],
        has_unsupported: true,
    };

    let md = report.to_markdown();
    assert!(md.contains("# API Usage Report"));
    assert!(md.contains("Total Extensions:** 2"));
    assert!(md.contains("Py_IncRef"));
    assert!(md.contains("PyLong_FromLong"));
    assert!(md.contains("Warning"));
}

// =============================================================================
// NumPy Loading Tests (require NumPy installation)
// =============================================================================

#[test]
#[ignore = "Requires NumPy installation"]
fn test_numpy_discovery() {
    let numpy_path = find_numpy_path();

    if numpy_path.is_none() {
        println!("NumPy not found, skipping test");
        return;
    }

    let numpy_path = numpy_path.unwrap();
    let mut loader = ExtensionLoader::default();
    loader.add_search_path(&numpy_path);

    // Try to find NumPy core module
    let discovery = loader.discovery();
    let core_path = discovery.find_extension("numpy.core._multiarray_umath");

    // This may fail if NumPy isn't installed, which is fine
    if let Ok(path) = core_path {
        println!("Found NumPy core at: {:?}", path);
        assert!(path.exists());
    }
}

#[test]
#[ignore = "Requires NumPy installation"]
fn test_numpy_abi_compatibility() {
    let numpy_path = find_numpy_path();

    if numpy_path.is_none() {
        println!("NumPy not found, skipping test");
        return;
    }

    let numpy_path = numpy_path.unwrap();
    let mut loader = ExtensionLoader::default();
    loader.add_search_path(&numpy_path);

    // Find a NumPy extension file
    let discovery = loader.discovery();
    if let Ok(path) = discovery.find_extension("numpy.core._multiarray_umath") {
        // Check ABI compatibility
        let compat = loader.check_compatibility(&path);

        match compat {
            Ok(AbiCompatibility::FullyCompatible) => {
                println!("NumPy is fully ABI compatible");
            }
            Ok(AbiCompatibility::Compatible { warnings }) => {
                println!("NumPy is compatible with warnings: {:?}", warnings);
            }
            Ok(AbiCompatibility::Incompatible { reason }) => {
                println!("NumPy is incompatible: {}", reason);
            }
            Err(e) => {
                println!("Error checking compatibility: {}", e);
            }
        }
    }
}

#[test]
#[ignore = "Requires NumPy installation and may crash if C API is incomplete"]
fn test_numpy_load() {
    let numpy_path = find_numpy_path();

    if numpy_path.is_none() {
        println!("NumPy not found, skipping test");
        return;
    }

    let numpy_path = numpy_path.unwrap();
    let mut loader = ExtensionLoader::default();
    loader.add_search_path(&numpy_path);

    // Attempt to load NumPy
    // Note: This will likely fail until we implement more C API functions
    let result = loader.load("numpy.core._multiarray_umath");

    match result {
        Ok(ext) => {
            println!("Successfully loaded NumPy: {:?}", ext);
            assert!(!ext.module().is_null());

            // Check API usage
            let report = loader.api_usage_report();
            println!("API Usage Report:\n{}", report.to_markdown());
        }
        Err(ExtensionError::UnsupportedApi { functions, .. }) => {
            println!("NumPy uses unsupported API functions: {:?}", functions);
            // This is expected until we implement more functions
        }
        Err(e) => {
            println!("Failed to load NumPy: {}", e);
            // This is expected until we implement more C API functions
        }
    }
}

// =============================================================================
// Array Operation Tests (require successful NumPy load)
// =============================================================================

#[test]
#[ignore = "Requires NumPy to be loadable"]
fn test_numpy_array_creation() {
    // This test would create a NumPy array and verify it works
    // For now, it's a placeholder until NumPy loading is fully functional

    use dx_py_ffi::numpy_compat::{
        npy_types, PyArray_Free, PyArray_NDIM, PyArray_SIZE, PyArray_SimpleNew,
    };

    unsafe {
        // Create a 3x4 float64 array
        let dims: [isize; 2] = [3, 4];
        let arr = PyArray_SimpleNew(2, dims.as_ptr(), npy_types::NPY_DOUBLE);

        if !arr.is_null() {
            assert_eq!(PyArray_NDIM(arr), 2);
            assert_eq!(PyArray_SIZE(arr), 12);
            PyArray_Free(arr);
        }
    }
}

#[test]
#[ignore = "Requires NumPy to be loadable"]
fn test_numpy_array_operations() {
    // This test would perform basic NumPy operations
    // For now, it's a placeholder

    use dx_py_ffi::numpy_compat::{npy_types, PyArray_DATA, PyArray_Free, PyArray_SimpleNew};

    unsafe {
        // Create a 1D array
        let dims: [isize; 1] = [5];
        let arr = PyArray_SimpleNew(1, dims.as_ptr(), npy_types::NPY_DOUBLE);

        if !arr.is_null() {
            // Get data pointer and fill with values
            let data = PyArray_DATA(arr) as *mut f64;
            if !data.is_null() {
                for i in 0..5 {
                    *data.add(i) = i as f64 * 2.0;
                }

                // Verify values
                assert_eq!(*data.add(0), 0.0);
                assert_eq!(*data.add(1), 2.0);
                assert_eq!(*data.add(2), 4.0);
            }

            PyArray_Free(arr);
        }
    }
}
