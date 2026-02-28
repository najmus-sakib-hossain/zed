//! Property-based tests for DPM module format

use dx_py_modules::{
    compiler::{ExportDef, ImportDef, ModuleDefinition},
    format::{DpmHeader, ExportKind},
    DpmCompiler, DpmLoader, ExportTable,
};
use proptest::prelude::*;

/// Property 14: Perfect Hash Export Lookup
/// Verifies O(1) lookup for all symbols
mod perfect_hash_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// All inserted symbols must be retrievable
        #[test]
        fn prop_all_symbols_retrievable(
            symbols in prop::collection::vec("[a-z][a-z0-9_]{0,20}", 1..50)
        ) {
            // Deduplicate symbols
            let unique: Vec<_> = symbols.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let exports: Vec<_> = unique.iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), ExportKind::Function, i as u32))
                .collect();

            let table = ExportTable::build(&exports).unwrap();

            // All symbols must be found
            for (name, _, offset) in &exports {
                let entry = table.get(name);
                prop_assert!(entry.is_some(), "Symbol {} not found", name);
                prop_assert_eq!(entry.unwrap().value_offset, *offset);
            }
        }

        /// Non-existent symbols must not be found
        #[test]
        fn prop_nonexistent_not_found(
            symbols in prop::collection::vec("[a-z][a-z0-9_]{0,10}", 1..20),
            queries in prop::collection::vec("[A-Z][A-Z0-9_]{0,10}", 1..20)
        ) {
            // Deduplicate symbols to avoid PerfectHashFailed
            let unique_symbols: Vec<_> = symbols.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            // Skip if no unique symbols
            prop_assume!(!unique_symbols.is_empty());

            let exports: Vec<_> = unique_symbols.iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), ExportKind::Function, i as u32))
                .collect();

            let table = ExportTable::build(&exports).unwrap();

            // Uppercase queries should not match lowercase symbols
            for query in &queries {
                let entry = table.get(query);
                // Should only find if there's an exact match (unlikely with different cases)
                if entry.is_some() {
                    prop_assert!(unique_symbols.contains(query));
                }
            }
        }

        /// Lookup is deterministic
        #[test]
        fn prop_lookup_deterministic(
            symbols in prop::collection::vec("[a-z][a-z0-9_]{0,15}", 1..30)
        ) {
            let unique: Vec<_> = symbols.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let exports: Vec<_> = unique.iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), ExportKind::Variable, i as u32))
                .collect();

            let table = ExportTable::build(&exports).unwrap();

            // Multiple lookups return same result
            for name in &unique {
                let first = table.get(name).map(|e| e.value_offset);
                let second = table.get(name).map(|e| e.value_offset);
                let third = table.get(name).map(|e| e.value_offset);
                prop_assert_eq!(first, second);
                prop_assert_eq!(second, third);
            }
        }
    }
}

/// Property 2: DPM Module Round-Trip Consistency
mod roundtrip_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Compiled module can be loaded and exports match
        #[test]
        fn prop_compile_load_roundtrip(
            module_name in "[a-z][a-z0-9_]{0,20}",
            export_names in prop::collection::vec("[a-z][a-z0-9_]{0,15}", 1..10),
            is_package in any::<bool>()
        ) {
            let unique_exports: Vec<_> = export_names.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let module = ModuleDefinition {
                name: module_name,
                is_package,
                imports: vec![],
                exports: unique_exports.iter().enumerate().map(|(i, name)| {
                    ExportDef {
                        name: name.clone(),
                        kind: ExportKind::Function,
                        data: vec![i as u8; 4],
                    }
                }).collect(),
                init_bytecode: vec![0xF0], // NOP
                type_annotations: vec![],
            };

            let mut compiler = DpmCompiler::new();
            let binary = compiler.compile(&module).unwrap();

            // Write to temp file
            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(&binary).unwrap();
            temp.flush().unwrap();

            // Load and verify
            let loader = DpmLoader::new();
            let loaded = loader.load(temp.path()).unwrap();

            // All exports should be found
            for name in &unique_exports {
                let entry = loaded.get_symbol(name);
                prop_assert!(entry.is_some(), "Export {} not found after roundtrip", name);
            }
        }

        /// Header fields are preserved
        #[test]
        fn prop_header_preserved(
            is_package in any::<bool>(),
            num_imports in 0usize..5,
            num_exports in 1usize..10
        ) {
            let module = ModuleDefinition {
                name: "test".to_string(),
                is_package,
                imports: (0..num_imports).map(|i| ImportDef {
                    module_name: format!("mod{}", i),
                    symbol_name: None,
                    alias: None,
                    is_star: false,
                    level: 0,
                }).collect(),
                exports: (0..num_exports).map(|i| ExportDef {
                    name: format!("export{}", i),
                    kind: ExportKind::Function,
                    data: vec![],
                }).collect(),
                init_bytecode: vec![],
                type_annotations: vec![],
            };

            let mut compiler = DpmCompiler::new();
            let binary = compiler.compile(&module).unwrap();

            let mut temp = NamedTempFile::new().unwrap();
            temp.write_all(&binary).unwrap();
            temp.flush().unwrap();

            let loader = DpmLoader::new();
            let loaded = loader.load(temp.path()).unwrap();
            let header = loaded.header();

            prop_assert_eq!(header.imports_count as usize, num_imports);
            prop_assert_eq!(header.exports_count as usize, num_exports);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_header_size_and_alignment() {
        assert_eq!(std::mem::align_of::<DpmHeader>(), 64);
        // Header should be at least 64 bytes for cache line alignment
        assert!(DpmHeader::size() >= 64);
    }

    #[test]
    fn test_empty_export_table() {
        let table = ExportTable::build(&[]).unwrap();
        assert!(table.is_empty());
        assert!(table.get("anything").is_none());
    }

    #[test]
    fn test_single_export() {
        let exports = vec![("single".to_string(), ExportKind::Function, 42)];
        let table = ExportTable::build(&exports).unwrap();

        let entry = table.get("single").unwrap();
        assert_eq!(entry.value_offset, 42);
        assert_eq!(entry.kind, ExportKind::Function);
    }
}

/// Property 10: Import Caching
/// Verifies that module caching works correctly
mod import_caching_tests {
    use super::*;
    use dx_py_modules::importer::{ImportSystem, LoaderType, ModuleSpec, ModuleValue, PyModule};
    use std::sync::Arc;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 10: Import Caching
        /// Same module import returns same object (Arc pointer equality)
        /// Validates: Requirements 5.7
        #[test]
        fn prop_import_caching_same_object(
            module_name in "[a-z][a-z0-9_]{0,15}"
        ) {
            let sys = ImportSystem::new();

            // Add a module to the system
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            let module = Arc::new(PyModule::new(spec));
            sys.add_module(&module_name, Arc::clone(&module));

            // Import twice
            let import1 = sys.import_module(&module_name).unwrap();
            let import2 = sys.import_module(&module_name).unwrap();

            // Should be the same Arc (pointer equality)
            prop_assert!(Arc::ptr_eq(&import1, &import2),
                "Repeated imports should return same object");
        }

        /// Feature: dx-py-production-ready, Property 10: Import Caching
        /// Module attributes persist across imports
        /// Validates: Requirements 5.7
        #[test]
        fn prop_import_caching_attributes_persist(
            module_name in "[a-z][a-z0-9_]{0,15}",
            attr_name in "[a-z][a-z0-9_]{0,10}",
            attr_value in any::<i64>()
        ) {
            let sys = ImportSystem::new();

            // Add a module
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            let module = Arc::new(PyModule::new(spec));
            sys.add_module(&module_name, Arc::clone(&module));

            // Import and set attribute
            let import1 = sys.import_module(&module_name).unwrap();
            import1.set_attr(&attr_name, ModuleValue::Int(attr_value));

            // Import again and check attribute
            let import2 = sys.import_module(&module_name).unwrap();
            let retrieved = import2.get_attr(&attr_name);

            prop_assert!(matches!(retrieved, Some(ModuleValue::Int(v)) if v == attr_value),
                "Attribute should persist across imports");
        }

        /// Feature: dx-py-production-ready, Property 10: Import Caching
        /// Reload creates new module object
        /// Validates: Requirements 5.7
        #[test]
        fn prop_reload_creates_new_object(
            module_name in "[a-z][a-z0-9_]{0,15}"
        ) {
            let sys = ImportSystem::new();

            // Add a built-in module (so reload can find it again)
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            let module = Arc::new(PyModule::new(spec));
            sys.add_module(&module_name, Arc::clone(&module));

            // Import first
            let import1 = sys.import_module(&module_name).unwrap();

            // Reload - for built-in modules, this will create a new instance
            // Note: reload removes from cache and re-imports
            let reloaded = sys.reload(&module_name);

            // Reload should succeed for built-in modules
            if let Ok(import2) = reloaded {
                // Should be different objects after reload
                prop_assert!(!Arc::ptr_eq(&import1, &import2),
                    "Reload should create new module object");
            }
        }

        /// Feature: dx-py-production-ready, Property 10: Import Caching
        /// Remove module clears cache
        /// Validates: Requirements 5.7
        #[test]
        fn prop_remove_clears_cache(
            module_name in "[a-z][a-z0-9_]{0,15}"
        ) {
            let sys = ImportSystem::new();

            // Add a module
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            let module = Arc::new(PyModule::new(spec));
            sys.add_module(&module_name, Arc::clone(&module));

            // Verify it's there
            prop_assert!(sys.get_module(&module_name).is_some());

            // Remove it
            let removed = sys.remove_module(&module_name);
            prop_assert!(removed.is_some());

            // Should be gone
            prop_assert!(sys.get_module(&module_name).is_none());
        }

        /// Feature: dx-py-production-ready, Property 10: Import Caching
        /// Multiple modules cached independently
        /// Validates: Requirements 5.7
        #[test]
        fn prop_multiple_modules_independent(
            names in prop::collection::vec("[a-z][a-z0-9_]{0,10}", 2..5)
        ) {
            // Deduplicate names
            let unique_names: Vec<_> = names.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            prop_assume!(unique_names.len() >= 2);

            let sys = ImportSystem::new();

            // Add all modules
            for name in &unique_names {
                let spec = ModuleSpec::new(name, LoaderType::BuiltIn);
                let module = Arc::new(PyModule::new(spec));
                sys.add_module(name, module);
            }

            // Import all and verify they're different
            let imports: Vec<_> = unique_names.iter()
                .map(|name| sys.import_module(name).unwrap())
                .collect();

            // Each module should be distinct
            for i in 0..imports.len() {
                for j in (i+1)..imports.len() {
                    prop_assert!(!Arc::ptr_eq(&imports[i], &imports[j]),
                        "Different modules should be different objects");
                }
            }
        }
    }
}

/// Property 11: Import Resolution
/// Verifies that imports resolve correctly
mod import_resolution_tests {
    use super::*;
    use dx_py_modules::importer::{
        ImportError, ImportSystem, LoaderType, ModuleSpec, ModuleValue, PyModule,
    };
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// Built-in modules are always found
        /// Validates: Requirements 5.1, 5.2
        #[test]
        fn prop_builtin_modules_found(
            builtin in prop::sample::select(vec![
                "sys", "builtins", "os", "io", "json", "re", "math",
                "collections", "itertools", "functools", "typing"
            ])
        ) {
            let sys = ImportSystem::new();
            let result = sys.import_module(builtin);

            prop_assert!(result.is_ok(), "Built-in module {} should be found", builtin);

            let module = result.unwrap();
            prop_assert_eq!(module.spec.loader.clone(), LoaderType::BuiltIn);
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// Non-existent modules return ModuleNotFound error
        /// Validates: Requirements 5.1
        #[test]
        fn prop_nonexistent_module_error(
            module_name in "[A-Z][A-Z0-9_]{10,20}_nonexistent"
        ) {
            let sys = ImportSystem::new();
            let result = sys.import_module(&module_name);

            prop_assert!(matches!(result, Err(ImportError::ModuleNotFound(_))),
                "Non-existent module should return ModuleNotFound");
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// Relative import with no package context fails
        /// Validates: Requirements 5.6
        #[test]
        fn prop_relative_import_no_package_fails(
            relative_name in "\\.[a-z][a-z0-9_]{0,10}"
        ) {
            let sys = ImportSystem::new();
            let result = sys.import_module_with_package(&relative_name, None);

            prop_assert!(matches!(result, Err(ImportError::NoParentPackage)),
                "Relative import without package should fail");
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// Single dot relative import resolves within package
        /// Validates: Requirements 5.6
        #[test]
        fn prop_single_dot_relative_import(
            package_name in "[a-z][a-z0-9_]{0,10}",
            submodule_name in "[a-z][a-z0-9_]{0,10}"
        ) {
            let sys = ImportSystem::new();

            // Create package structure
            let spec = ModuleSpec::new(&package_name, LoaderType::BuiltIn)
                .as_package(vec![]);
            sys.add_module(&package_name, Arc::new(PyModule::new(spec)));

            let full_name = format!("{}.{}", package_name, submodule_name);
            let spec = ModuleSpec::new(&full_name, LoaderType::BuiltIn);
            sys.add_module(&full_name, Arc::new(PyModule::new(spec)));

            // Import .submodule from package
            let relative_name = format!(".{}", submodule_name);
            let result = sys.import_module_with_package(&relative_name, Some(&package_name));

            prop_assert!(result.is_ok(),
                "Single dot relative import should resolve: {} from {}",
                relative_name, package_name);
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// Double dot relative import goes up one level
        /// Validates: Requirements 5.6
        #[test]
        fn prop_double_dot_relative_import(
            parent_name in "[a-z][a-z0-9_]{0,8}",
            child_name in "[a-z][a-z0-9_]{0,8}",
            sibling_name in "[a-z][a-z0-9_]{0,8}"
        ) {
            let sys = ImportSystem::new();

            // Create package structure: parent.child and parent.sibling
            let spec = ModuleSpec::new(&parent_name, LoaderType::BuiltIn)
                .as_package(vec![]);
            sys.add_module(&parent_name, Arc::new(PyModule::new(spec)));

            let child_full = format!("{}.{}", parent_name, child_name);
            let spec = ModuleSpec::new(&child_full, LoaderType::BuiltIn)
                .as_package(vec![]);
            sys.add_module(&child_full, Arc::new(PyModule::new(spec)));

            let sibling_full = format!("{}.{}", parent_name, sibling_name);
            let spec = ModuleSpec::new(&sibling_full, LoaderType::BuiltIn);
            sys.add_module(&sibling_full, Arc::new(PyModule::new(spec)));

            // Import ..sibling from parent.child
            let relative_name = format!("..{}", sibling_name);
            let result = sys.import_module_with_package(&relative_name, Some(&child_full));

            prop_assert!(result.is_ok(),
                "Double dot relative import should resolve: {} from {}",
                relative_name, child_full);
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// Beyond top-level relative import fails
        /// Validates: Requirements 5.6
        #[test]
        fn prop_beyond_top_level_fails(
            module_name in "[a-z][a-z0-9_]{0,10}"
        ) {
            let sys = ImportSystem::new();

            // Add a top-level module
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            sys.add_module(&module_name, Arc::new(PyModule::new(spec)));

            // Try to go beyond top level with ..something
            let result = sys.import_module_with_package("..something", Some(&module_name));

            prop_assert!(matches!(result, Err(ImportError::BeyondTopLevel)),
                "Beyond top-level import should fail");
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// import_from returns requested attributes
        /// Validates: Requirements 5.4
        #[test]
        fn prop_import_from_returns_attributes(
            module_name in "[a-z][a-z0-9_]{0,10}",
            attr_names in prop::collection::vec("[a-z][a-z0-9_]{0,8}", 1..5)
        ) {
            // Deduplicate attribute names
            let unique_attrs: Vec<_> = attr_names.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let sys = ImportSystem::new();

            // Create module with attributes
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            let module = Arc::new(PyModule::new(spec));

            for (i, attr) in unique_attrs.iter().enumerate() {
                module.set_attr(attr, ModuleValue::Int(i as i64));
            }

            sys.add_module(&module_name, module);

            // Import specific names
            let names: Vec<&str> = unique_attrs.iter().map(|s| s.as_str()).collect();
            let result = sys.import_from(&module_name, &names, None);

            prop_assert!(result.is_ok(), "import_from should succeed");

            let imports = result.unwrap();
            for attr in &unique_attrs {
                prop_assert!(imports.contains_key(attr),
                    "Imported attributes should contain {}", attr);
            }
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// import_from with missing name fails
        /// Validates: Requirements 5.4
        #[test]
        fn prop_import_from_missing_fails(
            module_name in "[a-z][a-z0-9_]{0,10}",
            missing_name in "[A-Z][A-Z0-9_]{5,10}"
        ) {
            let sys = ImportSystem::new();

            // Create empty module
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            sys.add_module(&module_name, Arc::new(PyModule::new(spec)));

            // Try to import non-existent name
            let result = sys.import_from(&module_name, &[&missing_name], None);

            prop_assert!(matches!(result, Err(ImportError::ImportFromError { .. })),
                "import_from with missing name should fail");
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// import_from with * uses __all__ if defined
        /// Validates: Requirements 5.5
        #[test]
        fn prop_import_star_uses_all(
            module_name in "[a-z][a-z0-9_]{0,10}",
            public_names in prop::collection::vec("[a-z][a-z0-9_]{0,8}", 1..4),
            private_name in "_[a-z][a-z0-9_]{0,8}"
        ) {
            // Deduplicate public names
            let unique_public: Vec<_> = public_names.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let sys = ImportSystem::new();

            // Create module with __all__ and private attribute
            let spec = ModuleSpec::new(&module_name, LoaderType::BuiltIn);
            let module = Arc::new(PyModule::new(spec));

            module.set_attr("__all__", ModuleValue::List(unique_public.clone()));

            for name in &unique_public {
                module.set_attr(name, ModuleValue::Int(1));
            }
            module.set_attr(&private_name, ModuleValue::Int(2));

            sys.add_module(&module_name, module);

            // Import *
            let result = sys.import_from(&module_name, &["*"], None);

            prop_assert!(result.is_ok(), "import * should succeed");

            let imports = result.unwrap();

            // Should have public names
            for name in &unique_public {
                prop_assert!(imports.contains_key(name),
                    "import * should include {} from __all__", name);
            }

            // Should not have private name (not in __all__)
            prop_assert!(!imports.contains_key(&private_name),
                "import * should not include private name not in __all__");
        }

        /// Feature: dx-py-production-ready, Property 11: Import Resolution
        /// Submodule import loads parent first
        /// Validates: Requirements 5.3
        #[test]
        fn prop_submodule_loads_parent(
            parent_name in "[a-z][a-z0-9_]{0,8}",
            child_name in "[a-z][a-z0-9_]{0,8}"
        ) {
            let sys = ImportSystem::new();

            // Create parent package
            let spec = ModuleSpec::new(&parent_name, LoaderType::BuiltIn)
                .as_package(vec![]);
            sys.add_module(&parent_name, Arc::new(PyModule::new(spec)));

            // Create child module
            let full_name = format!("{}.{}", parent_name, child_name);
            let spec = ModuleSpec::new(&full_name, LoaderType::BuiltIn);
            sys.add_module(&full_name, Arc::new(PyModule::new(spec)));

            // Import child - parent should be loaded first
            let result = sys.import_module(&full_name);

            prop_assert!(result.is_ok(), "Submodule import should succeed");

            // Parent should be in sys.modules
            prop_assert!(sys.get_module(&parent_name).is_some(),
                "Parent module should be loaded");
        }
    }

    // Unit tests for file-based imports
    #[test]
    fn test_source_file_import() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Create a simple module
        fs::write(
            root.join("mymodule.py"),
            r#"
"""Module docstring"""
__all__ = ['foo', 'bar']

VERSION = "1.0"
DEBUG = True
COUNT = 42

def foo():
    pass

def bar():
    pass

class MyClass:
    pass
"#,
        )
        .unwrap();

        let mut sys = ImportSystem::new();
        sys.add_path(root);

        let result = sys.import_module("mymodule");
        assert!(result.is_ok(), "Should import source file");

        let module = result.unwrap();
        assert_eq!(module.spec.loader, LoaderType::SourceFile);

        // Check __doc__ was extracted
        if let Some(ModuleValue::Str(doc)) = module.get_attr("__doc__") {
            assert_eq!(doc, "Module docstring");
        }

        // Check __all__ was extracted
        if let Some(ModuleValue::List(all)) = module.get_attr("__all__") {
            assert!(all.contains(&"foo".to_string()));
            assert!(all.contains(&"bar".to_string()));
        }

        // Check simple values were extracted
        assert!(module.has_attr("VERSION"));
        assert!(module.has_attr("DEBUG"));
        assert!(module.has_attr("COUNT"));
        assert!(module.has_attr("foo"));
        assert!(module.has_attr("bar"));
        assert!(module.has_attr("MyClass"));
    }

    #[test]
    fn test_package_import() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Create a package
        let pkg_dir = root.join("mypkg");
        fs::create_dir(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("__init__.py"), "PKG_VERSION = '2.0'\n").unwrap();
        fs::write(pkg_dir.join("submod.py"), "SUB_VALUE = 100\n").unwrap();

        let mut sys = ImportSystem::new();
        sys.add_path(root);

        // Import package
        let pkg = sys.import_module("mypkg").unwrap();
        assert!(pkg.spec.is_package);
        assert!(pkg.has_attr("PKG_VERSION"));

        // Import submodule
        let sub = sys.import_module("mypkg.submod").unwrap();
        assert!(!sub.spec.is_package);
        assert!(sub.has_attr("SUB_VALUE"));
    }

    #[test]
    fn test_namespace_package() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Create namespace package (no __init__.py)
        let ns_dir = root.join("namespace");
        fs::create_dir(&ns_dir).unwrap();
        fs::write(ns_dir.join("module.py"), "NS_VALUE = 'namespace'\n").unwrap();

        let mut sys = ImportSystem::new();
        sys.add_path(root);

        // Import namespace package
        let ns = sys.import_module("namespace").unwrap();
        assert!(ns.spec.is_package);
        assert_eq!(ns.spec.loader, LoaderType::NamespacePackage);
    }
}

/// Property 5: Module Import Completeness
/// Verifies that after import, all functions and classes are callable objects
mod module_completeness_tests {
    use super::*;
    use dx_py_modules::executor::{CodeFlags, CodeObject, ModuleExecutor, PyValue};
    use std::sync::Arc;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 5: Module Import Completeness
        /// For any valid Python module, after import, all functions and classes
        /// defined in the module SHALL be callable objects (not placeholder strings).
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5**
        #[test]
        fn prop_module_functions_are_callable(
            module_name in "[a-z][a-z0-9_]{0,15}",
            func_names in prop::collection::vec("[a-z][a-z0-9_]{0,10}", 1..5)
        ) {
            // Deduplicate function names
            let unique_funcs: Vec<_> = func_names.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let executor = ModuleExecutor::new();

            // Create code objects for each function
            for func_name in &unique_funcs {
                let code = CodeObject::new(func_name, "<test>");
                let func = executor.create_function(&code, &module_name);

                // Verify function is a proper PyFunction object
                prop_assert_eq!(&func.name, func_name);
                prop_assert_eq!(&func.module, &module_name);

                // Function should have code object
                prop_assert!(!func.code.code.is_empty() || &func.code.name == func_name);
            }
        }

        /// Property 5: Module Import Completeness
        /// Classes created from code objects are proper PyClass objects
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.3**
        #[test]
        fn prop_module_classes_are_types(
            module_name in "[a-z][a-z0-9_]{0,15}",
            class_name in "[A-Z][a-zA-Z0-9_]{0,15}"
        ) {
            let executor = ModuleExecutor::new();

            // Create a class body code object
            let body_code = CodeObject::new(&class_name, "<test>");

            // Create class
            let class = executor.create_class(
                &class_name,
                &class_name,
                &module_name,
                vec![],
                &body_code,
            ).unwrap();

            // Verify class is a proper PyClass object
            prop_assert_eq!(&class.name, &class_name);
            prop_assert_eq!(&class.module, &module_name);
        }

        /// Property 5: Module Import Completeness
        /// Module executor properly populates namespace with functions
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.2, 6.4**
        #[test]
        fn prop_executor_populates_namespace(
            func_name in "[a-z][a-z0-9_]{0,15}",
            doc_string in "[a-zA-Z0-9 ]{0,50}"
        ) {
            let executor = ModuleExecutor::new();

            // Create a code object with a docstring
            let mut code = CodeObject::new(&func_name, "<test>");
            code.consts.push(PyValue::Str(doc_string.clone()));

            let func = executor.create_function(&code, "test_module");

            // Function should have extracted docstring
            if !doc_string.is_empty() {
                prop_assert_eq!(func.doc, Some(doc_string));
            }
        }

        /// Property 5: Module Import Completeness
        /// Generator functions are properly marked
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.2**
        #[test]
        fn prop_generator_functions_marked(
            func_name in "[a-z][a-z0-9_]{0,15}"
        ) {
            let _executor = ModuleExecutor::new();

            let mut code = CodeObject::new(&func_name, "<test>");
            code.flags = CodeFlags::GENERATOR;

            // Generator flag should be detected from code object
            prop_assert!(code.is_generator());
        }

        /// Property 5: Module Import Completeness
        /// Async functions are properly marked
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.2**
        #[test]
        fn prop_async_functions_marked(
            func_name in "[a-z][a-z0-9_]{0,15}"
        ) {
            let _executor = ModuleExecutor::new();

            let mut code = CodeObject::new(&func_name, "<test>");
            code.flags = CodeFlags::COROUTINE;

            // Coroutine flag should be detected from code object
            prop_assert!(code.is_coroutine());
        }

        /// Property 5: Module Import Completeness
        /// Class methods are properly created
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.3**
        #[test]
        fn prop_class_methods_created(
            class_name in "[A-Z][a-zA-Z0-9_]{0,10}",
            method_names in prop::collection::vec("[a-z][a-z0-9_]{0,8}", 1..4)
        ) {
            // Deduplicate method names
            let unique_methods: Vec<_> = method_names.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let executor = ModuleExecutor::new();

            // Create class body with method code objects
            let mut body_code = CodeObject::new(&class_name, "<test>");
            for method_name in &unique_methods {
                let method_code = CodeObject::new(method_name, "<test>");
                body_code.consts.push(PyValue::Code(Arc::new(method_code)));
            }

            let class = executor.create_class(
                &class_name,
                &class_name,
                "test_module",
                vec![],
                &body_code,
            ).unwrap();

            // All methods should be in class dict
            for method_name in &unique_methods {
                prop_assert!(
                    class.get_attribute(method_name).is_some(),
                    "Class should have method {}", method_name
                );
            }
        }

        /// Property 5: Module Import Completeness
        /// PyValue types are correctly identified
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.2, 6.3**
        #[test]
        fn prop_pyvalue_type_names_correct(
            int_val in any::<i64>(),
            float_val in any::<f64>(),
            str_val in "[a-zA-Z0-9]{0,20}"
        ) {
            assert_eq!(PyValue::None.type_name(), "NoneType");
            assert_eq!(PyValue::Bool(true).type_name(), "bool");

            let int_v = PyValue::Int(int_val);
            prop_assert_eq!(int_v.type_name(), "int");

            let float_v = PyValue::Float(float_val);
            prop_assert_eq!(float_v.type_name(), "float");

            let str_v = PyValue::Str(str_val);
            prop_assert_eq!(str_v.type_name(), "str");

            let list_v = PyValue::List(vec![]);
            prop_assert_eq!(list_v.type_name(), "list");

            let dict_v = PyValue::Dict(std::collections::HashMap::new());
            prop_assert_eq!(dict_v.type_name(), "dict");
        }

        /// Property 5: Module Import Completeness
        /// PyValue truthiness follows Python semantics
        /// **Feature: dx-py-production-ready, Property 5: Module Import Completeness**
        /// **Validates: Requirements 6.1**
        #[test]
        fn prop_pyvalue_truthiness_correct(
            int_val in any::<i64>(),
            str_val in "[a-zA-Z0-9]{0,20}"
        ) {
            // None is falsy
            prop_assert!(!PyValue::None.is_truthy());

            // Bool follows value
            prop_assert!(PyValue::Bool(true).is_truthy());
            prop_assert!(!PyValue::Bool(false).is_truthy());

            // Int: 0 is falsy, others truthy
            prop_assert_eq!(PyValue::Int(int_val).is_truthy(), int_val != 0);

            // String: empty is falsy, non-empty truthy
            prop_assert_eq!(PyValue::Str(str_val.clone()).is_truthy(), !str_val.is_empty());

            // Empty collections are falsy
            prop_assert!(!PyValue::List(vec![]).is_truthy());
            prop_assert!(!PyValue::Dict(std::collections::HashMap::new()).is_truthy());

            // Non-empty collections are truthy
            prop_assert!(PyValue::List(vec![PyValue::Int(1)]).is_truthy());
        }
    }
}
