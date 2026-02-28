//! PyFunction - Python function type

use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use crate::pylist::PyValue;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Function flags
#[derive(Debug, Clone, Copy, Default)]
pub struct FunctionFlags {
    pub is_generator: bool,
    pub is_coroutine: bool,
    pub is_async_generator: bool,
    pub has_varargs: bool,
    pub has_kwargs: bool,
    pub is_method: bool,
    pub is_static: bool,
    pub is_classmethod: bool,
}

/// Parameter definition
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub kind: ParameterKind,
    pub default: Option<PyValue>,
    pub annotation: Option<String>,
}

/// Parameter kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterKind {
    Positional,
    PositionalOrKeyword,
    VarPositional, // *args
    KeywordOnly,
    VarKeyword, // **kwargs
}

/// Code object reference
#[derive(Debug, Clone)]
pub struct CodeRef {
    /// Bytecode offset in the module
    pub bytecode_offset: u32,
    /// Number of local variables
    pub num_locals: u16,
    /// Stack size needed
    pub stack_size: u16,
    /// Number of arguments
    pub num_args: u8,
    /// Number of keyword-only arguments
    pub num_kwonly_args: u8,
}

/// Python function object
pub struct PyFunction {
    /// Object header
    pub header: PyObjectHeader,
    /// Function name
    pub name: String,
    /// Qualified name (module.class.func)
    pub qualname: String,
    /// Module name
    pub module: Option<String>,
    /// Source filename (for debugging)
    pub filename: Option<String>,
    /// First line number (for debugging)
    pub firstlineno: u32,
    /// Docstring
    pub doc: Option<String>,
    /// Parameters
    pub params: Vec<Parameter>,
    /// Return type annotation
    pub return_annotation: Option<String>,
    /// Code reference
    pub code: CodeRef,
    /// Default argument values
    pub defaults: Vec<PyValue>,
    /// Keyword-only defaults
    pub kwdefaults: Vec<(String, PyValue)>,
    /// Closure variables
    pub closure: Vec<PyValue>,
    /// Global namespace reference
    pub globals: Option<Arc<crate::PyDict>>,
    /// Function flags
    pub flags: FunctionFlags,
    /// Call counter for JIT compilation triggering
    /// When this reaches the threshold (100), the function becomes a candidate for JIT compilation
    call_count: AtomicU64,
}

/// JIT compilation threshold - functions called more than this many times
/// are candidates for JIT compilation
pub const JIT_COMPILATION_THRESHOLD: u64 = 100;

impl PyFunction {
    /// Create a new function
    pub fn new(name: impl Into<String>, code: CodeRef, params: Vec<Parameter>) -> Self {
        let name = name.into();
        Self {
            header: PyObjectHeader::new(TypeTag::Function, ObjectFlags::CALLABLE),
            qualname: name.clone(),
            name,
            module: None,
            filename: None,
            firstlineno: 0,
            doc: None,
            params,
            return_annotation: None,
            code,
            defaults: Vec::new(),
            kwdefaults: Vec::new(),
            closure: Vec::new(),
            globals: None,
            flags: FunctionFlags::default(),
            call_count: AtomicU64::new(0),
        }
    }

    /// Record a function call and return the new call count
    /// This is used for hot function detection - when the count reaches
    /// JIT_COMPILATION_THRESHOLD (100), the function should be JIT compiled
    #[inline]
    pub fn record_call(&self) -> u64 {
        self.call_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Get the current call count
    #[inline]
    pub fn get_call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Check if this function is hot (called enough times to warrant JIT compilation)
    #[inline]
    pub fn is_hot(&self) -> bool {
        self.get_call_count() >= JIT_COMPILATION_THRESHOLD
    }

    /// Check if this function just became hot (exactly at the threshold)
    /// This is useful for triggering JIT compilation exactly once
    #[inline]
    pub fn just_became_hot(&self) -> bool {
        self.get_call_count() == JIT_COMPILATION_THRESHOLD
    }

    /// Set the module
    pub fn with_module(mut self, module: impl Into<String>) -> Self {
        self.module = Some(module.into());
        self
    }

    /// Set the filename
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Set the first line number
    pub fn with_firstlineno(mut self, lineno: u32) -> Self {
        self.firstlineno = lineno;
        self
    }

    /// Set the docstring
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = Some(doc.into());
        self
    }

    /// Set default values
    pub fn with_defaults(mut self, defaults: Vec<PyValue>) -> Self {
        self.defaults = defaults;
        self
    }

    /// Set closure
    pub fn with_closure(mut self, closure: Vec<PyValue>) -> Self {
        self.closure = closure;
        self
    }

    /// Set globals
    pub fn with_globals(mut self, globals: Arc<crate::PyDict>) -> Self {
        self.globals = Some(globals);
        self
    }

    /// Get the number of required positional arguments
    pub fn num_required_args(&self) -> usize {
        self.params
            .iter()
            .filter(|p| {
                matches!(p.kind, ParameterKind::Positional | ParameterKind::PositionalOrKeyword)
                    && p.default.is_none()
            })
            .count()
    }

    /// Get the maximum number of positional arguments
    pub fn max_positional_args(&self) -> Option<usize> {
        if self.flags.has_varargs {
            None
        } else {
            Some(
                self.params
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.kind,
                            ParameterKind::Positional | ParameterKind::PositionalOrKeyword
                        )
                    })
                    .count(),
            )
        }
    }

    /// Check if function accepts keyword arguments
    pub fn accepts_kwargs(&self) -> bool {
        self.flags.has_kwargs
    }

    /// Get parameter by name
    pub fn get_param(&self, name: &str) -> Option<&Parameter> {
        self.params.iter().find(|p| p.name == name)
    }

    /// Get default value for parameter
    pub fn get_default(&self, param_index: usize) -> Option<&PyValue> {
        let num_positional = self
            .params
            .iter()
            .filter(|p| {
                matches!(p.kind, ParameterKind::Positional | ParameterKind::PositionalOrKeyword)
            })
            .count();

        let defaults_start = num_positional.saturating_sub(self.defaults.len());

        if param_index >= defaults_start {
            self.defaults.get(param_index - defaults_start)
        } else {
            None
        }
    }
}

impl std::fmt::Debug for PyFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<function {} at {:p}>", self.qualname, self)
    }
}

/// Built-in function (implemented in Rust)
pub struct PyBuiltinFunction {
    /// Object header
    pub header: PyObjectHeader,
    /// Function name
    pub name: String,
    /// Module name
    pub module: Option<String>,
    /// Docstring
    pub doc: Option<String>,
    /// The actual function pointer
    pub func: Arc<dyn Fn(&[PyValue]) -> Result<PyValue, crate::RuntimeError> + Send + Sync>,
}

impl PyBuiltinFunction {
    /// Create a new builtin function
    pub fn new<F>(name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&[PyValue]) -> Result<PyValue, crate::RuntimeError> + Send + Sync + 'static,
    {
        Self {
            header: PyObjectHeader::new(TypeTag::Function, ObjectFlags::CALLABLE),
            name: name.into(),
            module: None,
            doc: None,
            func: Arc::new(func),
        }
    }

    /// Call the function
    pub fn call(&self, args: &[PyValue]) -> Result<PyValue, crate::RuntimeError> {
        (self.func)(args)
    }
}

impl std::fmt::Debug for PyBuiltinFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<built-in function {}>", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_creation() {
        let code = CodeRef {
            bytecode_offset: 0,
            num_locals: 2,
            stack_size: 4,
            num_args: 2,
            num_kwonly_args: 0,
        };

        let params = vec![
            Parameter {
                name: "x".into(),
                kind: ParameterKind::PositionalOrKeyword,
                default: None,
                annotation: None,
            },
            Parameter {
                name: "y".into(),
                kind: ParameterKind::PositionalOrKeyword,
                default: Some(PyValue::Int(0)),
                annotation: None,
            },
        ];

        let func = PyFunction::new("add", code, params);
        assert_eq!(func.name, "add");
        assert_eq!(func.num_required_args(), 1);
    }

    #[test]
    fn test_builtin_function() {
        let func = PyBuiltinFunction::new("len", |args| match args.first() {
            Some(PyValue::Str(s)) => Ok(PyValue::Int(s.len() as i64)),
            _ => Err(crate::RuntimeError::type_error("str", "other")),
        });

        let result = func.call(&[PyValue::Str(Arc::from("hello"))]).unwrap();
        if let PyValue::Int(len) = result {
            assert_eq!(len, 5);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_call_counter_initial_value() {
        let code = CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 1,
            num_args: 0,
            num_kwonly_args: 0,
        };

        let func = PyFunction::new("test", code, vec![]);
        assert_eq!(func.get_call_count(), 0);
        assert!(!func.is_hot());
    }

    #[test]
    fn test_call_counter_increment() {
        let code = CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 1,
            num_args: 0,
            num_kwonly_args: 0,
        };

        let func = PyFunction::new("test", code, vec![]);
        
        // Record some calls
        assert_eq!(func.record_call(), 1);
        assert_eq!(func.record_call(), 2);
        assert_eq!(func.record_call(), 3);
        
        assert_eq!(func.get_call_count(), 3);
        assert!(!func.is_hot());
    }

    #[test]
    fn test_hot_function_detection() {
        let code = CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 1,
            num_args: 0,
            num_kwonly_args: 0,
        };

        let func = PyFunction::new("test", code, vec![]);
        
        // Call 99 times - not hot yet
        for _ in 0..99 {
            func.record_call();
        }
        assert_eq!(func.get_call_count(), 99);
        assert!(!func.is_hot());
        assert!(!func.just_became_hot());
        
        // 100th call - now hot
        func.record_call();
        assert_eq!(func.get_call_count(), 100);
        assert!(func.is_hot());
        assert!(func.just_became_hot());
        
        // 101st call - still hot but not "just became hot"
        func.record_call();
        assert_eq!(func.get_call_count(), 101);
        assert!(func.is_hot());
        assert!(!func.just_became_hot());
    }

    #[test]
    fn test_jit_compilation_threshold() {
        // Verify the threshold constant is 100
        assert_eq!(JIT_COMPILATION_THRESHOLD, 100);
    }
}
