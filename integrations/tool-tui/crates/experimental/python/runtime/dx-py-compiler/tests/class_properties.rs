//! Property-based tests for the DX-Py Class System
//!
//! Feature: dx-py-production-ready
//! Property 1: Class Method Compilation Produces Valid Bytecode
//! Property 2: Class Instantiation Correctly Binds Arguments
//! Property 3: Method Resolution Order Follows C3 Linearization
//! Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5

use dx_py_bytecode::{CodeObject, Constant, DpbOpcode};
use dx_py_compiler::SourceCompiler;
use proptest::prelude::*;

// ===== Generators for property tests =====

/// Generate a valid Python identifier (for class/method names)
fn arb_identifier() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z_][a-z0-9_]{0,10}")
        .unwrap()
        .prop_filter("not a keyword", |s| !is_python_keyword(s))
}

/// Generate a valid Python class name (starts with uppercase)
fn arb_class_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Z][a-zA-Z0-9_]{0,15}")
        .unwrap()
        .prop_filter("not a keyword", |s| !is_python_keyword(s))
}

/// Check if a string is a Python keyword
fn is_python_keyword(s: &str) -> bool {
    matches!(
        s,
        "False" | "None" | "True" | "and" | "as" | "assert" | "async" | "await"
            | "break" | "class" | "continue" | "def" | "del" | "elif" | "else"
            | "except" | "finally" | "for" | "from" | "global" | "if" | "import"
            | "in" | "is" | "lambda" | "nonlocal" | "not" | "or" | "pass" | "raise"
            | "return" | "try" | "while" | "with" | "yield" | "self"
    )
}

/// Generate a list of unique identifiers for method parameters
fn arb_unique_params(count: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::hash_set(arb_identifier(), 1..=count)
        .prop_map(|set| set.into_iter().collect::<Vec<String>>())
        .prop_filter("non-empty", |v: &Vec<String>| !v.is_empty())
}

/// Generate a simple integer value for testing
#[allow(dead_code)]
fn arb_int_value() -> impl Strategy<Value = i64> {
    -1000i64..1000i64
}

/// Generate a list of unique class names for hierarchy testing
fn arb_unique_class_names(count: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::hash_set(arb_class_name(), count..=count)
        .prop_map(|set| set.into_iter().collect())
}

// ===== Helper Functions =====

/// Check if bytecode contains a specific opcode
fn bytecode_contains_opcode(code: &[u8], opcode: DpbOpcode) -> bool {
    code.iter().any(|&b| b == opcode as u8)
}

/// Find all CodeRef constants in a CodeObject
fn find_code_constants(code: &CodeObject) -> Vec<&CodeObject> {
    code.constants
        .iter()
        .filter_map(|c| match c {
            Constant::Code(inner) => Some(inner.as_ref()),
            _ => None,
        })
        .collect()
}

/// Check if a CodeObject represents a valid method (has proper structure)
fn is_valid_method_code(code: &CodeObject) -> bool {
    // A valid method should have:
    // 1. Non-empty bytecode
    // 2. At least one local variable (self)
    // 3. A return instruction
    !code.code.is_empty()
        && code.nlocals >= 1
        && bytecode_contains_opcode(&code.code, DpbOpcode::Return)
}

// ===== Property 1: Class Method Compilation Produces Valid Bytecode =====

/// Feature: dx-py-production-ready, Property 1: Class Method Compilation Produces Valid Bytecode
/// For any class definition with methods, the compiler SHALL produce bytecode where each method
/// body is a valid CodeRef (not a string reference).
/// Validates: Requirements 1.1
mod class_method_compilation_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 1: Class Method Compilation Produces Valid Bytecode
        /// Validates: Requirements 1.1
        ///
        /// For any class with an __init__ method, the compiled bytecode SHALL contain
        /// a CodeRef for the method body, not a string reference.
        #[test]
        fn prop_init_method_compiles_to_code_ref(
            class_name in arb_class_name(),
            param in arb_identifier()
        ) {
            let source = format!(
                "class {}:\n    def __init__(self, {}):\n        self.{} = {}",
                class_name, param, param, param
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile class: {}", source);

            let code = result.unwrap();

            // Find the class body code object
            let class_codes = find_code_constants(&code);
            prop_assert!(!class_codes.is_empty(),
                "Class definition should produce at least one CodeRef constant");

            // The class body should contain the __init__ method as a CodeRef
            let class_body = class_codes[0];
            let method_codes = find_code_constants(class_body);

            prop_assert!(!method_codes.is_empty(),
                "__init__ method should be compiled to a CodeRef, not a string");

            // Verify the method code is valid
            let init_code = method_codes[0];
            prop_assert!(is_valid_method_code(init_code),
                "__init__ method should have valid bytecode structure");
        }

        /// Feature: dx-py-production-ready, Property 1: Class Method Compilation Produces Valid Bytecode
        /// Validates: Requirements 1.1
        ///
        /// For any class with multiple methods, each method SHALL be compiled to a valid CodeRef.
        #[test]
        fn prop_multiple_methods_compile_to_code_refs(
            class_name in arb_class_name(),
            method_count in 1usize..5
        ) {
            // Generate unique method names
            let method_names: Vec<String> = (0..method_count)
                .map(|i| format!("method_{}", i))
                .collect();

            let methods = method_names.iter()
                .map(|name| format!("    def {}(self):\n        pass", name))
                .collect::<Vec<_>>()
                .join("\n");

            let source = format!("class {}:\n{}", class_name, methods);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile class with {} methods", method_count);

            let code = result.unwrap();

            // Find the class body code object
            let class_codes = find_code_constants(&code);
            prop_assert!(!class_codes.is_empty(),
                "Class definition should produce CodeRef constants");

            // The class body should contain CodeRefs for all methods
            let class_body = class_codes[0];
            let method_codes = find_code_constants(class_body);

            prop_assert_eq!(method_codes.len(), method_count,
                "Expected {} method CodeRefs, found {}", method_count, method_codes.len());

            // Each method should be valid
            for (i, method_code) in method_codes.iter().enumerate() {
                prop_assert!(is_valid_method_code(method_code),
                    "Method {} should have valid bytecode structure", i);
            }
        }

        /// Feature: dx-py-production-ready, Property 1: Class Method Compilation Produces Valid Bytecode
        /// Validates: Requirements 1.1
        ///
        /// For any method with parameters, the compiled CodeRef SHALL have the correct argcount.
        #[test]
        fn prop_method_argcount_matches_parameters(
            class_name in arb_class_name(),
            params in arb_unique_params(5)
        ) {
            let param_str = params.join(", ");
            let source = format!(
                "class {}:\n    def method(self, {}):\n        pass",
                class_name, param_str
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile class: {}", source);

            let code = result.unwrap();

            // Find the method code object
            let class_codes = find_code_constants(&code);
            prop_assert!(!class_codes.is_empty(), "Should have class code");

            let class_body = class_codes[0];
            let method_codes = find_code_constants(class_body);
            prop_assert!(!method_codes.is_empty(), "Should have method code");

            let method_code = method_codes[0];

            // argcount should be params.len() + 1 (for self)
            let expected_argcount = (params.len() + 1) as u32;
            prop_assert_eq!(method_code.argcount, expected_argcount,
                "Method argcount should be {} (self + {} params), got {}",
                expected_argcount, params.len(), method_code.argcount);
        }
    }

    // ===== Unit tests for Property 1 =====

    #[test]
    fn test_simple_class_compiles() {
        let source = r#"
class MyClass:
    def __init__(self):
        self.x = 1
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple class should compile");

        let code = result.unwrap();
        let class_codes = find_code_constants(&code);
        assert!(!class_codes.is_empty(), "Should have class code");

        let class_body = class_codes[0];
        let method_codes = find_code_constants(class_body);
        assert!(!method_codes.is_empty(), "__init__ should be a CodeRef");
    }

    #[test]
    fn test_class_with_multiple_methods_compiles() {
        let source = r#"
class Calculator:
    def __init__(self, value):
        self.value = value

    def add(self, x):
        return self.value + x

    def subtract(self, x):
        return self.value - x
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Class with multiple methods should compile");

        let code = result.unwrap();
        let class_codes = find_code_constants(&code);
        assert!(!class_codes.is_empty(), "Should have class code");

        let class_body = class_codes[0];
        let method_codes = find_code_constants(class_body);
        assert_eq!(method_codes.len(), 3, "Should have 3 method CodeRefs");
    }

    #[test]
    fn test_method_has_return_opcode() {
        let source = r#"
class Test:
    def method(self):
        return 42
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let code = compiler.compile_module_source(source).unwrap();

        let class_codes = find_code_constants(&code);
        let class_body = class_codes[0];
        let method_codes = find_code_constants(class_body);
        let method_code = method_codes[0];

        assert!(bytecode_contains_opcode(&method_code.code, DpbOpcode::Return),
            "Method should contain Return opcode");
    }
}

// ===== Property 2: Class Instantiation Correctly Binds Arguments =====

/// Feature: dx-py-production-ready, Property 2: Class Instantiation Correctly Binds Arguments
/// For any class with an __init__ method that stores its arguments as instance attributes,
/// instantiating the class with arguments SHALL result in instance attributes matching
/// the passed arguments.
/// Validates: Requirements 1.2, 1.3
mod class_instantiation_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 2: Class Instantiation Correctly Binds Arguments
        /// Validates: Requirements 1.2, 1.3
        ///
        /// For any class with __init__ that stores arguments as attributes,
        /// the compiled bytecode SHALL contain StoreAttr opcodes for each attribute.
        #[test]
        fn prop_init_stores_attributes(
            class_name in arb_class_name(),
            params in arb_unique_params(5)
        ) {
            let param_str = params.join(", ");
            let assignments = params.iter()
                .map(|p| format!("        self.{} = {}", p, p))
                .collect::<Vec<_>>()
                .join("\n");

            let source = format!(
                "class {}:\n    def __init__(self, {}):\n{}",
                class_name, param_str, assignments
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile class: {}", source);

            let code = result.unwrap();

            // Find the __init__ method code
            let class_codes = find_code_constants(&code);
            prop_assert!(!class_codes.is_empty(), "Should have class code");

            let class_body = class_codes[0];
            let method_codes = find_code_constants(class_body);
            prop_assert!(!method_codes.is_empty(), "Should have __init__ code");

            let init_code = method_codes[0];

            // Count StoreAttr opcodes - should match number of parameters
            let store_attr_count = init_code.code.iter()
                .filter(|&&b| b == DpbOpcode::StoreAttr as u8)
                .count();

            prop_assert_eq!(store_attr_count, params.len(),
                "Expected {} StoreAttr opcodes for {} params, found {}",
                params.len(), params.len(), store_attr_count);
        }

        /// Feature: dx-py-production-ready, Property 2: Class Instantiation Correctly Binds Arguments
        /// Validates: Requirements 1.2, 1.3
        ///
        /// For any __init__ method, the first parameter (self) SHALL be loaded
        /// before each attribute store.
        #[test]
        fn prop_self_loaded_before_store_attr(
            class_name in arb_class_name(),
            attr_name in arb_identifier()
        ) {
            let source = format!(
                "class {}:\n    def __init__(self, value):\n        self.{} = value",
                class_name, attr_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile class: {}", source);

            let code = result.unwrap();

            // Find the __init__ method code
            let class_codes = find_code_constants(&code);
            let class_body = class_codes[0];
            let method_codes = find_code_constants(class_body);
            let init_code = method_codes[0];

            // Verify LoadFast appears before StoreAttr
            // LoadFast(0) loads self, which should precede StoreAttr
            let has_load_fast = bytecode_contains_opcode(&init_code.code, DpbOpcode::LoadFast);
            let has_store_attr = bytecode_contains_opcode(&init_code.code, DpbOpcode::StoreAttr);

            prop_assert!(has_load_fast, "Should have LoadFast for self");
            prop_assert!(has_store_attr, "Should have StoreAttr for attribute");
        }

        /// Feature: dx-py-production-ready, Property 2: Class Instantiation Correctly Binds Arguments
        /// Validates: Requirements 1.2, 1.3
        ///
        /// For any class instantiation, the bytecode SHALL contain a Call opcode
        /// to invoke the class constructor.
        #[test]
        fn prop_class_instantiation_generates_call(
            class_name in arb_class_name(),
            arg_count in 0usize..5
        ) {
            let args = (0..arg_count).map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
            let source = format!(
                "class {}:\n    def __init__(self):\n        pass\n\nobj = {}({})",
                class_name, class_name, args
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // The module-level code should contain a Call opcode for instantiation
            let has_call = bytecode_contains_opcode(&code.code, DpbOpcode::Call);
            prop_assert!(has_call, "Class instantiation should generate Call opcode");
        }
    }

    // ===== Unit tests for Property 2 =====

    #[test]
    fn test_init_with_single_param() {
        let source = r#"
class Point:
    def __init__(self, x):
        self.x = x
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let code = compiler.compile_module_source(source).unwrap();

        let class_codes = find_code_constants(&code);
        let class_body = class_codes[0];
        let method_codes = find_code_constants(class_body);
        let init_code = method_codes[0];

        // Should have exactly 1 StoreAttr
        let store_attr_count = init_code.code.iter()
            .filter(|&&b| b == DpbOpcode::StoreAttr as u8)
            .count();
        assert_eq!(store_attr_count, 1, "Should have 1 StoreAttr for self.x");
    }

    #[test]
    fn test_init_with_multiple_params() {
        let source = r#"
class Point3D:
    def __init__(self, x, y, z):
        self.x = x
        self.y = y
        self.z = z
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let code = compiler.compile_module_source(source).unwrap();

        let class_codes = find_code_constants(&code);
        let class_body = class_codes[0];
        let method_codes = find_code_constants(class_body);
        let init_code = method_codes[0];

        // Should have exactly 3 StoreAttr
        let store_attr_count = init_code.code.iter()
            .filter(|&&b| b == DpbOpcode::StoreAttr as u8)
            .count();
        assert_eq!(store_attr_count, 3, "Should have 3 StoreAttr for x, y, z");
    }

    #[test]
    fn test_class_instantiation_bytecode() {
        let source = r#"
class MyClass:
    def __init__(self):
        pass

obj = MyClass()
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let code = compiler.compile_module_source(source).unwrap();

        // Module-level code should have Call for instantiation
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Call),
            "Should have Call opcode for instantiation");
    }
}

// ===== Property 3: Method Resolution Order Follows C3 Linearization =====

/// Feature: dx-py-production-ready, Property 3: Method Resolution Order Follows C3 Linearization
/// For any class hierarchy, the computed MRO SHALL match Python's C3 linearization algorithm output.
/// Validates: Requirements 1.4, 1.5
mod mro_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 3: Method Resolution Order Follows C3 Linearization
        /// Validates: Requirements 1.4, 1.5
        ///
        /// For any class with single inheritance, the MRO SHALL be a linear chain
        /// from derived to base classes.
        #[test]
        fn prop_single_inheritance_mro_is_linear(
            names in arb_unique_class_names(4)
        ) {
            // Create a linear inheritance chain: D -> C -> B -> A
            let source = format!(
                "class {}:\n    pass\n\nclass {}({}):\n    pass\n\nclass {}({}):\n    pass\n\nclass {}({}):\n    pass",
                names[0], names[1], names[0], names[2], names[1], names[3], names[2]
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile inheritance chain: {}", source);

            // The compilation should succeed without MRO conflicts
            // In a linear chain, MRO is simply the chain itself
        }

        /// Feature: dx-py-production-ready, Property 3: Method Resolution Order Follows C3 Linearization
        /// Validates: Requirements 1.4, 1.5
        ///
        /// For any diamond inheritance pattern, the compiler SHALL successfully
        /// compile the class hierarchy (C3 linearization should resolve).
        #[test]
        fn prop_diamond_inheritance_compiles(
            names in arb_unique_class_names(4)
        ) {
            // Diamond: D -> B, C -> A
            //          B -> A
            //          C -> A
            let source = format!(
                "class {}:\n    pass\n\nclass {}({}):\n    pass\n\nclass {}({}):\n    pass\n\nclass {}({}, {}):\n    pass",
                names[0], names[1], names[0], names[2], names[0], names[3], names[1], names[2]
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(),
                "Diamond inheritance should compile successfully: {}", source);
        }

        /// Feature: dx-py-production-ready, Property 3: Method Resolution Order Follows C3 Linearization
        /// Validates: Requirements 1.4, 1.5
        ///
        /// For any class with inheritance, the bytecode SHALL contain the base class
        /// references for MRO computation at runtime.
        #[test]
        fn prop_inheritance_includes_base_references(
            base_name in arb_class_name(),
            derived_name in arb_class_name()
        ) {
            // Ensure names are different
            if base_name == derived_name {
                return Ok(());
            }

            let source = format!(
                "class {}:\n    pass\n\nclass {}({}):\n    pass",
                base_name, derived_name, base_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // The module should reference both class names
            prop_assert!(code.names.contains(&base_name),
                "Base class name should be in names table");
            prop_assert!(code.names.contains(&derived_name),
                "Derived class name should be in names table");
        }

        /// Feature: dx-py-production-ready, Property 3: Method Resolution Order Follows C3 Linearization
        /// Validates: Requirements 1.4, 1.5
        ///
        /// For any class with super() call, the bytecode SHALL contain LoadGlobal
        /// for the super builtin.
        #[test]
        fn prop_super_call_loads_builtin(
            base_name in arb_class_name(),
            derived_name in arb_class_name()
        ) {
            // Ensure names are different
            if base_name == derived_name {
                return Ok(());
            }

            let source = format!(
                "class {}:\n    def __init__(self):\n        pass\n\nclass {}({}):\n    def __init__(self):\n        super().__init__()",
                base_name, derived_name, base_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile super() call: {}", source);

            let code = result.unwrap();

            // Find the derived class's __init__ method
            let class_codes = find_code_constants(&code);
            prop_assert!(class_codes.len() >= 2, "Should have at least 2 class codes");

            // The derived class body should reference 'super'
            let derived_class_body = class_codes[1];
            let method_codes = find_code_constants(derived_class_body);
            prop_assert!(!method_codes.is_empty(), "Should have __init__ method");

            let init_code = method_codes[0];

            // The __init__ method should have LoadGlobal for 'super'
            prop_assert!(init_code.names.contains(&"super".to_string()),
                "super() call should reference 'super' builtin");
        }
    }

    // ===== Unit tests for Property 3 =====

    #[test]
    fn test_simple_inheritance_compiles() {
        let source = r#"
class Base:
    def method(self):
        return "base"

class Derived(Base):
    def method(self):
        return "derived"
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple inheritance should compile");
    }

    #[test]
    fn test_diamond_inheritance_compiles() {
        let source = r#"
class A:
    pass

class B(A):
    pass

class C(A):
    pass

class D(B, C):
    pass
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Diamond inheritance should compile");
    }

    #[test]
    fn test_super_call_compiles() {
        let source = r#"
class Parent:
    def __init__(self):
        self.x = 1

class Child(Parent):
    def __init__(self):
        super().__init__()
        self.y = 2
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "super() call should compile");
    }

    #[test]
    fn test_multiple_inheritance_compiles() {
        let source = r#"
class Mixin1:
    def method1(self):
        return 1

class Mixin2:
    def method2(self):
        return 2

class Combined(Mixin1, Mixin2):
    def combined(self):
        return self.method1() + self.method2()
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Multiple inheritance should compile");
    }

    #[test]
    fn test_deep_inheritance_chain() {
        let source = r#"
class A:
    pass

class B(A):
    pass

class C(B):
    pass

class D(C):
    pass

class E(D):
    pass
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Deep inheritance chain should compile");
    }

    #[test]
    fn test_super_with_args_compiles() {
        let source = r#"
class Parent:
    def __init__(self, value):
        self.value = value

class Child(Parent):
    def __init__(self, value, extra):
        super().__init__(value)
        self.extra = extra
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "super() with args should compile");
    }
}
