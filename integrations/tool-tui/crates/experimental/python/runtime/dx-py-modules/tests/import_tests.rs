//! Tests for the module import system

use dx_py_modules::importer::{
    ImportError, ImportSystem, LoaderType, ModuleSpec, ModuleValue, PyModule,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Create a test directory structure with Python modules
fn create_test_modules(dir: &TempDir) -> PathBuf {
    let root = dir.path().to_path_buf();

    // Create a simple module
    fs::write(root.join("simple.py"), "x = 42\n").unwrap();

    // Create a package with __init__.py
    let pkg_dir = root.join("mypackage");
    fs::create_dir(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("__init__.py"), "__all__ = ['submod']\nversion = '1.0'\n").unwrap();
    fs::write(pkg_dir.join("submod.py"), "y = 100\n").unwrap();

    // Create a nested package
    let nested_dir = pkg_dir.join("nested");
    fs::create_dir(&nested_dir).unwrap();
    fs::write(nested_dir.join("__init__.py"), "").unwrap();
    fs::write(nested_dir.join("deep.py"), "z = 'deep'\n").unwrap();

    // Create a namespace package (no __init__.py)
    let ns_dir = root.join("namespace_pkg");
    fs::create_dir(&ns_dir).unwrap();
    fs::write(ns_dir.join("module.py"), "ns_value = 'namespace'\n").unwrap();

    root
}

#[test]
fn test_import_system_creation() {
    let sys = ImportSystem::new();
    assert!(sys.get_path().is_empty());
}

#[test]
fn test_add_path() {
    let mut sys = ImportSystem::new();
    sys.add_path("/usr/lib/python3.12");
    sys.add_path("/home/user/project");

    assert_eq!(sys.get_path().len(), 2);
}

#[test]
fn test_import_builtin_module() {
    let sys = ImportSystem::new();

    // Built-in modules should be found
    let result = sys.import_module("sys");
    assert!(result.is_ok());

    let module = result.unwrap();
    assert_eq!(module.spec.name, "sys");
    assert_eq!(module.spec.loader, LoaderType::BuiltIn);
}

#[test]
fn test_import_nonexistent_module() {
    let sys = ImportSystem::new();

    let result = sys.import_module("nonexistent_module_xyz");
    assert!(matches!(result, Err(ImportError::ModuleNotFound(_))));
}

#[test]
fn test_module_spec_creation() {
    let spec = ModuleSpec::new("mymodule", LoaderType::SourceFile);
    assert_eq!(spec.name, "mymodule");
    assert!(!spec.is_package);
    assert!(spec.parent.is_none());
}

#[test]
fn test_module_spec_with_parent() {
    let spec = ModuleSpec::new("parent.child", LoaderType::SourceFile);
    assert_eq!(spec.name, "parent.child");
    assert_eq!(spec.parent, Some("parent".to_string()));
}

#[test]
fn test_module_spec_as_package() {
    let spec = ModuleSpec::new("mypackage", LoaderType::SourceFile)
        .as_package(vec![PathBuf::from("/path/to/mypackage")]);

    assert!(spec.is_package);
    assert!(spec.submodule_search_locations.is_some());
}

#[test]
fn test_pymodule_creation() {
    let spec = ModuleSpec::new("test_module", LoaderType::SourceFile)
        .with_origin(PathBuf::from("/path/to/test_module.py"));

    let module = PyModule::new(spec);

    // Check standard attributes
    assert!(module.has_attr("__name__"));
    assert!(module.has_attr("__file__"));
    assert!(module.has_attr("__loader__"));

    if let Some(ModuleValue::Str(name)) = module.get_attr("__name__") {
        assert_eq!(name, "test_module");
    } else {
        panic!("Expected __name__ to be a string");
    }
}

#[test]
fn test_pymodule_package_attributes() {
    let spec = ModuleSpec::new("mypackage", LoaderType::SourceFile)
        .with_origin(PathBuf::from("/path/to/mypackage/__init__.py"))
        .as_package(vec![PathBuf::from("/path/to/mypackage")]);

    let module = PyModule::new(spec);

    // Packages should have __path__
    assert!(module.has_attr("__path__"));

    // __package__ should be the package name
    if let Some(ModuleValue::Str(pkg)) = module.get_attr("__package__") {
        assert_eq!(pkg, "mypackage");
    } else {
        panic!("Expected __package__ to be a string");
    }
}

#[test]
fn test_pymodule_set_get_attr() {
    let spec = ModuleSpec::new("test", LoaderType::BuiltIn);
    let module = PyModule::new(spec);

    module.set_attr("custom_attr", ModuleValue::Int(42));

    if let Some(ModuleValue::Int(val)) = module.get_attr("custom_attr") {
        assert_eq!(val, 42);
    } else {
        panic!("Expected custom_attr to be an int");
    }
}

#[test]
fn test_pymodule_exports() {
    let spec = ModuleSpec::new("test", LoaderType::BuiltIn);
    let module = PyModule::new(spec);

    // Set __all__
    module.set_attr("__all__", ModuleValue::List(vec!["foo".to_string(), "bar".to_string()]));
    module.set_attr("foo", ModuleValue::Int(1));
    module.set_attr("bar", ModuleValue::Int(2));
    module.set_attr("_private", ModuleValue::Int(3));

    let exports = module.get_exports();
    assert!(exports.contains(&"foo".to_string()));
    assert!(exports.contains(&"bar".to_string()));
    assert!(!exports.contains(&"_private".to_string()));
}

#[test]
fn test_pymodule_exports_without_all() {
    let spec = ModuleSpec::new("test", LoaderType::BuiltIn);
    let module = PyModule::new(spec);

    module.set_attr("public_func", ModuleValue::Int(1));
    module.set_attr("_private_func", ModuleValue::Int(2));

    let exports = module.get_exports();
    assert!(exports.contains(&"public_func".to_string()));
    assert!(!exports.contains(&"_private_func".to_string()));
}

#[test]
fn test_import_caching() {
    let sys = ImportSystem::new();

    // Import the same module twice
    let module1 = sys.import_module("sys").unwrap();
    let module2 = sys.import_module("sys").unwrap();

    // Should be the same Arc
    assert!(Arc::ptr_eq(&module1, &module2));
}

#[test]
fn test_resolve_relative_import_single_dot() {
    let sys = ImportSystem::new();

    // Create a mock package structure
    let spec = ModuleSpec::new("mypackage", LoaderType::BuiltIn).as_package(vec![]);
    let pkg = Arc::new(PyModule::new(spec));
    sys.add_module("mypackage", pkg);

    let spec = ModuleSpec::new("mypackage.submod", LoaderType::BuiltIn);
    let submod = Arc::new(PyModule::new(spec));
    sys.add_module("mypackage.submod", submod);

    // Import .submod from mypackage
    let result = sys.import_module_with_package(".submod", Some("mypackage"));
    assert!(result.is_ok());
}

#[test]
fn test_resolve_relative_import_double_dot() {
    let sys = ImportSystem::new();

    // Create a mock package structure: parent.child
    let spec = ModuleSpec::new("parent", LoaderType::BuiltIn).as_package(vec![]);
    sys.add_module("parent", Arc::new(PyModule::new(spec)));

    let spec = ModuleSpec::new("parent.child", LoaderType::BuiltIn).as_package(vec![]);
    sys.add_module("parent.child", Arc::new(PyModule::new(spec)));

    let spec = ModuleSpec::new("parent.sibling", LoaderType::BuiltIn);
    sys.add_module("parent.sibling", Arc::new(PyModule::new(spec)));

    // Import ..sibling from parent.child
    let result = sys.import_module_with_package("..sibling", Some("parent.child"));
    assert!(result.is_ok());
}

#[test]
fn test_relative_import_no_package() {
    let sys = ImportSystem::new();

    // Relative import without package context should fail
    let result = sys.import_module_with_package(".submod", None);
    assert!(matches!(result, Err(ImportError::NoParentPackage)));
}

#[test]
fn test_relative_import_beyond_top_level() {
    let sys = ImportSystem::new();

    // Add the toplevel module so it can be found
    let spec = ModuleSpec::new("toplevel", LoaderType::BuiltIn);
    sys.add_module("toplevel", Arc::new(PyModule::new(spec)));

    // Trying to go beyond top-level should fail
    // ..something from "toplevel" means go up 1 level (to nothing) then import something
    // With 2 dots from a single-level package, we try to go up 1 level which leaves us with empty parts
    let result = sys.import_module_with_package("..something", Some("toplevel"));

    // The result should be BeyondTopLevel error
    match &result {
        Err(ImportError::BeyondTopLevel) => (), // Expected
        Err(e) => panic!("Expected BeyondTopLevel, got: {:?}", e),
        Ok(m) => panic!("Expected error, got module: {:?}", m.spec.name),
    }
}

#[test]
fn test_import_from() {
    let sys = ImportSystem::new();

    // Create a module with some attributes
    let spec = ModuleSpec::new("testmod", LoaderType::BuiltIn);
    let module = Arc::new(PyModule::new(spec));
    module.set_attr("foo", ModuleValue::Int(1));
    module.set_attr("bar", ModuleValue::Str("hello".to_string()));
    sys.add_module("testmod", module);

    // Import specific names
    let result = sys.import_from("testmod", &["foo", "bar"], None);
    assert!(result.is_ok());

    let imports = result.unwrap();
    assert!(imports.contains_key("foo"));
    assert!(imports.contains_key("bar"));
}

#[test]
fn test_import_from_star() {
    let sys = ImportSystem::new();

    // Create a module with __all__
    let spec = ModuleSpec::new("testmod", LoaderType::BuiltIn);
    let module = Arc::new(PyModule::new(spec));
    module.set_attr("__all__", ModuleValue::List(vec!["exported".to_string()]));
    module.set_attr("exported", ModuleValue::Int(42));
    module.set_attr("_private", ModuleValue::Int(0));
    sys.add_module("testmod", module);

    // Import *
    let result = sys.import_from("testmod", &["*"], None);
    assert!(result.is_ok());

    let imports = result.unwrap();
    assert!(imports.contains_key("exported"));
    assert!(!imports.contains_key("_private"));
}

#[test]
fn test_import_from_missing_name() {
    let sys = ImportSystem::new();

    let spec = ModuleSpec::new("testmod", LoaderType::BuiltIn);
    let module = Arc::new(PyModule::new(spec));
    sys.add_module("testmod", module);

    // Try to import a name that doesn't exist
    let result = sys.import_from("testmod", &["nonexistent"], None);
    assert!(matches!(result, Err(ImportError::ImportFromError { .. })));
}

#[test]
fn test_reload_module() {
    let sys = ImportSystem::new();

    // Import a built-in module first
    let v1 = sys.import_module("sys").unwrap();

    // Reload - since it's a built-in, it will be found again
    let v2 = sys.reload("sys").unwrap();

    // Should be different instances (reload creates new module)
    assert!(!Arc::ptr_eq(&v1, &v2));
}

#[test]
fn test_remove_module() {
    let sys = ImportSystem::new();

    let spec = ModuleSpec::new("removable", LoaderType::BuiltIn);
    sys.add_module("removable", Arc::new(PyModule::new(spec)));

    assert!(sys.get_module("removable").is_some());

    let removed = sys.remove_module("removable");
    assert!(removed.is_some());
    assert!(sys.get_module("removable").is_none());
}

#[test]
fn test_path_finder_source_file() {
    let dir = TempDir::new().unwrap();
    let root = create_test_modules(&dir);

    let mut sys = ImportSystem::new();
    sys.add_path(&root);

    // Import simple.py
    let result = sys.import_module("simple");
    assert!(result.is_ok());

    let module = result.unwrap();
    assert_eq!(module.spec.loader, LoaderType::SourceFile);
    assert!(module.spec.origin.is_some());
}

#[test]
fn test_path_finder_package() {
    let dir = TempDir::new().unwrap();
    let root = create_test_modules(&dir);

    let mut sys = ImportSystem::new();
    sys.add_path(&root);

    // Import mypackage
    let result = sys.import_module("mypackage");
    assert!(result.is_ok());

    let module = result.unwrap();
    assert!(module.spec.is_package);
    assert!(module.spec.submodule_search_locations.is_some());
}

#[test]
fn test_path_finder_submodule() {
    let dir = TempDir::new().unwrap();
    let root = create_test_modules(&dir);

    let mut sys = ImportSystem::new();
    sys.add_path(&root);

    // Import mypackage.submod
    let result = sys.import_module("mypackage.submod");
    assert!(result.is_ok());

    let module = result.unwrap();
    assert_eq!(module.spec.name, "mypackage.submod");
    assert!(!module.spec.is_package);
}

#[test]
fn test_path_finder_nested_package() {
    let dir = TempDir::new().unwrap();
    let root = create_test_modules(&dir);

    let mut sys = ImportSystem::new();
    sys.add_path(&root);

    // Import mypackage.nested.deep
    let result = sys.import_module("mypackage.nested.deep");
    assert!(result.is_ok());

    let module = result.unwrap();
    assert_eq!(module.spec.name, "mypackage.nested.deep");
}

#[test]
fn test_path_finder_namespace_package() {
    let dir = TempDir::new().unwrap();
    let root = create_test_modules(&dir);

    let mut sys = ImportSystem::new();
    sys.add_path(&root);

    // Import namespace_pkg (no __init__.py)
    let result = sys.import_module("namespace_pkg");
    assert!(result.is_ok());

    let module = result.unwrap();
    assert_eq!(module.spec.loader, LoaderType::NamespacePackage);
    assert!(module.spec.is_package);
}

#[test]
fn test_loader_types() {
    assert_eq!(LoaderType::SourceFile, LoaderType::SourceFile);
    assert_ne!(LoaderType::SourceFile, LoaderType::BuiltIn);
}

#[test]
fn test_module_value_variants() {
    let none = ModuleValue::None;
    let bool_val = ModuleValue::Bool(true);
    let int_val = ModuleValue::Int(42);
    let float_val = ModuleValue::Float(3.125);
    let str_val = ModuleValue::Str("hello".to_string());
    let list_val = ModuleValue::List(vec!["a".to_string(), "b".to_string()]);

    // Just verify they can be created and cloned
    let _ = none.clone();
    let _ = bool_val.clone();
    let _ = int_val.clone();
    let _ = float_val.clone();
    let _ = str_val.clone();
    let _ = list_val.clone();
}
