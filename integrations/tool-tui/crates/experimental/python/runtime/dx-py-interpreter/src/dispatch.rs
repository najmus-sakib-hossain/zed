//! Bytecode dispatch loop

#![allow(clippy::result_large_err)]

use crate::opcodes::Opcode;
use crate::{InterpreterError, InterpreterResult};
use dashmap::DashMap;
use dx_py_core::pydict::PyKey;
use dx_py_core::pyframe::PyFrame;
use dx_py_core::pyfunction::{ParameterKind, PyBuiltinFunction, PyFunction};
use dx_py_core::pylist::{PyModule, PyValue};
use dx_py_core::pygenerator::{PyCoroutine, PyGenerator};
use dx_py_core::{PyDict, PyIterator};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Result of dispatching an opcode
pub enum DispatchResult {
    Continue,
    Return(PyValue),
    Yield(PyValue),
    Exception(PyValue),
}

/// Bytecode dispatcher
pub struct Dispatcher {
    /// Constants pool
    constants: Vec<PyValue>,
    /// Names pool
    names: Vec<String>,
    /// Bytecode
    code: Vec<u8>,
    /// Global namespace
    globals: Arc<PyDict>,
    /// Builtin functions
    builtins: HashMap<String, Arc<PyBuiltinFunction>>,
    /// Module cache (sys.modules equivalent)
    modules: Arc<DashMap<String, Arc<PyModule>>>,
    /// Module search paths (sys.path equivalent)
    sys_path: Arc<Vec<PathBuf>>,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new(code: Vec<u8>, constants: Vec<PyValue>, names: Vec<String>) -> Self {
        Self {
            code,
            constants,
            names,
            globals: Arc::new(PyDict::new()),
            builtins: HashMap::new(),
            modules: Arc::new(DashMap::new()),
            sys_path: Arc::new(Vec::new()),
        }
    }

    /// Create a new dispatcher with globals and builtins
    pub fn with_globals(
        code: Vec<u8>,
        constants: Vec<PyValue>,
        names: Vec<String>,
        globals: Arc<PyDict>,
        builtins: HashMap<String, Arc<PyBuiltinFunction>>,
    ) -> Self {
        Self {
            code,
            constants,
            names,
            globals,
            builtins,
            modules: Arc::new(DashMap::new()),
            sys_path: Arc::new(Vec::new()),
        }
    }

    /// Create a new dispatcher with globals, builtins, and module cache
    pub fn with_modules(
        code: Vec<u8>,
        constants: Vec<PyValue>,
        names: Vec<String>,
        globals: Arc<PyDict>,
        builtins: HashMap<String, Arc<PyBuiltinFunction>>,
        modules: Arc<DashMap<String, Arc<PyModule>>>,
        sys_path: Arc<Vec<PathBuf>>,
    ) -> Self {
        Self {
            code,
            constants,
            names,
            globals,
            builtins,
            modules,
            sys_path,
        }
    }

    /// Execute bytecode in a frame
    pub fn execute(&self, frame: &mut PyFrame) -> InterpreterResult<PyValue> {
        loop {
            if frame.ip >= self.code.len() {
                break;
            }

            let opcode_byte = self.code[frame.ip];
            frame.ip += 1;

            let opcode = Opcode::from_byte(opcode_byte).ok_or_else(|| {
                InterpreterError::Runtime(format!("Unknown opcode: 0x{:02x}", opcode_byte))
            })?;

            // Read argument based on opcode's argument size
            let arg = match opcode.arg_size() {
                0 => None,
                1 => {
                    if frame.ip < self.code.len() {
                        let arg = self.code[frame.ip] as usize;
                        frame.ip += 1;
                        Some(arg)
                    } else {
                        None
                    }
                }
                2 => {
                    if frame.ip + 1 < self.code.len() {
                        let arg =
                            u16::from_le_bytes([self.code[frame.ip], self.code[frame.ip + 1]]);
                        frame.ip += 2;
                        Some(arg as usize)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            match self.dispatch(frame, opcode, arg)? {
                DispatchResult::Continue => continue,
                DispatchResult::Return(value) => {
                    // Check if there are any finally blocks that need to execute
                    // before returning
                    if let Some(finally_block) = frame.find_finally_handler() {
                        let handler = finally_block.handler;
                        let level = finally_block.level;
                        // Pop the finally block so we don't re-enter it
                        frame.pop_block();
                        frame.unwind_to(level);
                        // Push the return value first, then a marker (Int(2)) to indicate "return pending"
                        // Stack layout: [return_value, marker]
                        // Marker values: 0 = normal exit, 1 = exception, 2 = return, 3 = break, 4 = continue
                        frame.push(value);
                        frame.push(PyValue::Int(2)); // 2 = return pending
                        frame.ip = handler;
                        continue;
                    }
                    return Ok(value);
                }
                DispatchResult::Yield(value) => return Ok(value),
                DispatchResult::Exception(exc) => {
                    // Find the nearest exception handler (Except or Finally)
                    if let Some(block) = frame.find_exception_handler() {
                        let handler = block.handler;
                        let level = block.level;
                        let block_type = block.block_type;

                        // Pop the block we're jumping to
                        frame.pop_block();
                        frame.unwind_to(level);

                        match block_type {
                            dx_py_core::pyframe::BlockType::Except => {
                                // Push exception for the except handler
                                frame.push(exc);
                            }
                            dx_py_core::pyframe::BlockType::Finally => {
                                // Push exception so finally can re-raise it
                                frame.push(exc);
                            }
                            _ => {
                                // For other block types (Loop, With), continue unwinding
                                // Re-raise the exception to find the next handler
                                frame.push(exc.clone());
                                continue;
                            }
                        }

                        frame.ip = handler;
                        continue;
                    } else {
                        return Err(InterpreterError::Exception(exc));
                    }
                }
            }
        }
        Ok(PyValue::None)
    }

    /// Dispatch a single opcode
    fn dispatch(
        &self,
        frame: &mut PyFrame,
        opcode: Opcode,
        arg: Option<usize>,
    ) -> InterpreterResult<DispatchResult> {
        match opcode {
            // Stack operations
            Opcode::Nop => {}
            Opcode::Pop => {
                frame.pop();
            }
            Opcode::Dup => {
                let top = frame.peek().clone();
                frame.push(top);
            }
            Opcode::DupTwo => {
                let a = frame.peek_n(1).clone();
                let b = frame.peek_n(0).clone();
                frame.push(a);
                frame.push(b);
            }
            Opcode::Swap => {
                let a = frame.pop();
                let b = frame.pop();
                frame.push(a);
                frame.push(b);
            }
            Opcode::RotN => {
                let n = arg.unwrap_or(2);
                // Rotate top N items
                if n > 1 {
                    let top = frame.pop();
                    let mut items = Vec::with_capacity(n - 1);
                    for _ in 0..(n - 1) {
                        items.push(frame.pop());
                    }
                    frame.push(top);
                    for item in items.into_iter().rev() {
                        frame.push(item);
                    }
                }
            }
            Opcode::Copy => {
                let n = arg.unwrap_or(1);
                let value = frame.peek_n(n - 1).clone();
                frame.push(value);
            }

            // Load operations
            Opcode::LoadConst => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("LOAD_CONST requires argument".into())
                })?;
                let value = self.constants.get(idx).cloned().unwrap_or(PyValue::None);
                frame.push(value);
            }
            Opcode::LoadFast => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("LOAD_FAST requires argument".into())
                })?;
                let value = frame.get_local(idx).clone();
                frame.push(value);
            }
            Opcode::LoadName | Opcode::LoadGlobal => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("LOAD_NAME/LOAD_GLOBAL requires argument".into())
                })?;
                let name = self.names.get(idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", idx))
                })?;

                // First check globals
                if let Ok(value) = self.globals.getitem(&PyKey::Str(Arc::from(name.as_str()))) {
                    frame.push(value);
                }
                // Then check builtins
                else if let Some(builtin) = self.builtins.get(name) {
                    frame.push(PyValue::Builtin(builtin.clone()));
                } else {
                    return Err(InterpreterError::NameError(format!(
                        "name '{}' is not defined",
                        name
                    )));
                }
            }
            Opcode::LoadAttr => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("LOAD_ATTR requires argument".into())
                })?;
                let name = self.names.get(idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", idx))
                })?;
                let obj = frame.pop();

                let result = match &obj {
                    PyValue::Instance(instance) => {
                        // Use instance.get_attr() for lookup which follows Python's attribute resolution:
                        // 1. Data descriptors from type's MRO
                        // 2. Instance __dict__ or __slots__
                        // 3. Non-data descriptors and other class attributes from MRO
                        if let Some(value) = instance.get_attr(name) {
                            // If it's a function from the class, create a bound method
                            match &value {
                                PyValue::Function(func) => {
                                    // Create bound method with instance
                                    PyValue::BoundMethod(
                                        dx_py_core::types::BoundMethod::bind_instance(
                                            PyValue::Function(Arc::clone(func)),
                                            Arc::clone(instance),
                                        ),
                                    )
                                }
                                _ => value,
                            }
                        } else {
                            return Err(InterpreterError::AttributeError(format!(
                                "'{}' object has no attribute '{}'",
                                instance.class_name(),
                                name
                            )));
                        }
                    }
                    PyValue::Type(class) => {
                        // Get attribute from class (type object)
                        if let Some(value) = class.get_attr_from_mro(name) {
                            value
                        } else {
                            return Err(InterpreterError::AttributeError(format!(
                                "type object '{}' has no attribute '{}'",
                                class.name, name
                            )));
                        }
                    }
                    PyValue::Module(module) => {
                        // Get attribute from module's dict (DashMap)
                        module.dict.get(name).map(|v| v.value().clone()).ok_or_else(|| {
                            InterpreterError::AttributeError(format!(
                                "module '{}' has no attribute '{}'",
                                module.name, name
                            ))
                        })?
                    }
                    PyValue::Dict(dict) => {
                        // For dict, support common methods by returning bound methods
                        match name.as_str() {
                            "keys" | "values" | "items" | "get" | "pop" | "update" | "clear"
                            | "copy" | "setdefault" | "popitem" => {
                                PyValue::BoundMethod(
                                    dx_py_core::types::BoundMethod::bind_dict(
                                        Arc::clone(dict),
                                        name.clone(),
                                    ),
                                )
                            }
                            _ => {
                                return Err(InterpreterError::AttributeError(format!(
                                    "'dict' object has no attribute '{}'",
                                    name
                                )));
                            }
                        }
                    }
                    PyValue::List(list) => {
                        // For list, support common methods by returning bound methods
                        match name.as_str() {
                            "append" | "extend" | "insert" | "remove" | "pop" | "clear"
                            | "index" | "count" | "sort" | "reverse" | "copy" => {
                                PyValue::BoundMethod(
                                    dx_py_core::types::BoundMethod::bind_list(
                                        Arc::clone(list),
                                        name.clone(),
                                    ),
                                )
                            }
                            _ => {
                                return Err(InterpreterError::AttributeError(format!(
                                    "'list' object has no attribute '{}'",
                                    name
                                )));
                            }
                        }
                    }
                    PyValue::Str(s) => {
                        // For str, support common methods by returning bound methods
                        match name.as_str() {
                            "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "split" | "join" 
                            | "replace" | "startswith" | "endswith" | "find" | "format" => {
                                PyValue::BoundMethod(
                                    dx_py_core::types::BoundMethod::bind_string(
                                        Arc::clone(s),
                                        name.clone(),
                                    ),
                                )
                            }
                            _ => {
                                return Err(InterpreterError::AttributeError(format!(
                                    "'str' object has no attribute '{}'",
                                    name
                                )));
                            }
                        }
                    }
                    _ => {
                        return Err(InterpreterError::AttributeError(format!(
                            "'{}' object has no attribute '{}'",
                            obj.type_name(),
                            name
                        )));
                    }
                };

                frame.push(result);
            }
            Opcode::LoadMethod => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("LOAD_METHOD requires argument".into())
                })?;
                let name = self.names.get(idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", idx))
                })?;

                // LOAD_METHOD is an optimization for method calls
                // Stack: [obj] -> [method, self] or [NULL, bound_method]
                //
                // If we find an unbound method (function in class), we push:
                //   [method_function, self] - so CALL_METHOD can call directly
                // If we find a bound method or other callable, we push:
                //   [NULL, bound_method] - so CALL_METHOD knows to call without extra self

                let obj = frame.pop();

                match &obj {
                    PyValue::Instance(instance) => {
                        // Look up the method in the class hierarchy
                        // First check instance dict (for bound methods stored on instance)
                        if let Some(value) = instance.dict.get(name) {
                            // Found in instance dict - it's already bound or a regular value
                            frame.push(PyValue::None); // NULL marker
                            frame.push(value.clone());
                        } else if let Some(class_attr) = instance.class.get_attr_from_mro(name) {
                            // Found in class - check if it's a function (unbound method)
                            match &class_attr {
                                PyValue::Function(_) => {
                                    // Unbound method - push method and self separately
                                    // CALL_METHOD will prepend self to args
                                    frame.push(class_attr);
                                    frame.push(PyValue::Instance(Arc::clone(instance)));
                                }
                                PyValue::BoundMethod(_) => {
                                    // Already bound - push NULL and the bound method
                                    frame.push(PyValue::None);
                                    frame.push(class_attr);
                                }
                                PyValue::Builtin(_) => {
                                    // Builtin method - push method and self
                                    frame.push(class_attr);
                                    frame.push(PyValue::Instance(Arc::clone(instance)));
                                }
                                _ => {
                                    // Other callable - push NULL and the value
                                    frame.push(PyValue::None);
                                    frame.push(class_attr);
                                }
                            }
                        } else {
                            return Err(InterpreterError::AttributeError(format!(
                                "'{}' object has no attribute '{}'",
                                instance.class_name(),
                                name
                            )));
                        }
                    }
                    PyValue::Type(class) => {
                        // Method lookup on a class (e.g., MyClass.method)
                        if let Some(value) = class.get_attr_from_mro(name) {
                            frame.push(PyValue::None);
                            frame.push(value);
                        } else {
                            return Err(InterpreterError::AttributeError(format!(
                                "type object '{}' has no attribute '{}'",
                                class.name, name
                            )));
                        }
                    }
                    PyValue::Module(module) => {
                        // Method lookup on a module (DashMap)
                        let value =
                            module.dict.get(name).map(|v| v.value().clone()).ok_or_else(|| {
                                InterpreterError::AttributeError(format!(
                                    "module '{}' has no attribute '{}'",
                                    module.name, name
                                ))
                            })?;
                        frame.push(PyValue::None);
                        frame.push(value);
                    }
                    PyValue::Str(s) => {
                        // For str, support common methods by returning bound methods
                        match name.as_str() {
                            "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "split" | "join" 
                            | "replace" | "startswith" | "endswith" | "find" | "format" => {
                                let bound_method = PyValue::BoundMethod(
                                    dx_py_core::types::BoundMethod::bind_string(
                                        Arc::clone(s),
                                        name.clone(),
                                    ),
                                );
                                frame.push(PyValue::None);
                                frame.push(bound_method);
                            }
                            _ => {
                                return Err(InterpreterError::AttributeError(format!(
                                    "'str' object has no attribute '{}'",
                                    name
                                )));
                            }
                        }
                    }
                    PyValue::List(list) => {
                        // For list, support common methods by returning bound methods
                        match name.as_str() {
                            "append" | "extend" | "insert" | "remove" | "pop" | "clear"
                            | "index" | "count" | "sort" | "reverse" | "copy" => {
                                let bound_method = PyValue::BoundMethod(
                                    dx_py_core::types::BoundMethod::bind_list(
                                        Arc::clone(list),
                                        name.clone(),
                                    ),
                                );
                                frame.push(PyValue::None);
                                frame.push(bound_method);
                            }
                            _ => {
                                return Err(InterpreterError::AttributeError(format!(
                                    "'list' object has no attribute '{}'",
                                    name
                                )));
                            }
                        }
                    }
                    PyValue::Dict(dict) => {
                        // For dict, support common methods by returning bound methods
                        match name.as_str() {
                            "keys" | "values" | "items" | "get" | "pop" | "update" | "clear"
                            | "copy" | "setdefault" | "popitem" => {
                                let bound_method = PyValue::BoundMethod(
                                    dx_py_core::types::BoundMethod::bind_dict(
                                        Arc::clone(dict),
                                        name.clone(),
                                    ),
                                );
                                frame.push(PyValue::None);
                                frame.push(bound_method);
                            }
                            _ => {
                                return Err(InterpreterError::AttributeError(format!(
                                    "'dict' object has no attribute '{}'",
                                    name
                                )));
                            }
                        }
                    }
                    _ => {
                        return Err(InterpreterError::AttributeError(format!(
                            "'{}' object has no attribute '{}'",
                            obj.type_name(),
                            name
                        )));
                    }
                }
            }
            Opcode::LoadDeref | Opcode::LoadClassDeref => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("LOAD_DEREF requires argument".into())
                })?;
                // Load value from cell at specified index
                // Cell variables are indexed first, followed by free variables
                let value = frame.get_deref(idx);
                frame.push(value);
            }
            Opcode::LoadSubscr => {
                let index = frame.pop();
                let container = frame.pop();
                let result = self.subscript(&container, &index)?;
                frame.push(result);
            }

            // Store operations
            Opcode::StoreName | Opcode::StoreGlobal => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("STORE_NAME/STORE_GLOBAL requires argument".into())
                })?;
                let name = self.names.get(idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", idx))
                })?;
                let value = frame.pop();
                self.globals.setitem(PyKey::Str(Arc::from(name.as_str())), value);
            }
            Opcode::StoreFast => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("STORE_FAST requires argument".into())
                })?;
                let value = frame.pop();
                frame.set_local(idx, value);
            }
            Opcode::StoreAttr => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("STORE_ATTR requires argument".into())
                })?;
                let name = self.names.get(idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", idx))
                })?;
                let obj = frame.pop();
                let value = frame.pop();

                match &obj {
                    PyValue::Instance(instance) => {
                        // Store attribute in instance's dict
                        // This follows Python's attribute setting behavior:
                        // - Check for data descriptors with __set__ in class
                        // - Otherwise store in instance __dict__
                        instance.set_attr(name.clone(), value);
                    }
                    PyValue::Type(class) => {
                        // Store attribute on the class itself
                        class.set_attr(name.clone(), value);
                    }
                    PyValue::Module(module) => {
                        // Store attribute in module's dict (DashMap)
                        module.dict.insert(name.clone(), value);
                    }
                    _ => {
                        return Err(InterpreterError::AttributeError(format!(
                            "'{}' object attribute '{}' is read-only",
                            obj.type_name(),
                            name
                        )));
                    }
                }
            }
            Opcode::StoreDeref => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("STORE_DEREF requires argument".into())
                })?;
                let value = frame.pop();
                // Store value into cell at specified index
                // Cell variables are indexed first, followed by free variables
                frame.set_deref(idx, value);
            }
            Opcode::StoreSubscr => {
                let index = frame.pop();
                let container = frame.pop();
                let value = frame.pop();
                self.store_subscript(&container, &index, value)?;
            }

            // Delete operations
            Opcode::DeleteFast => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("DELETE_FAST requires argument".into())
                })?;
                frame.set_local(idx, PyValue::None);
            }
            Opcode::DeleteGlobal => {
                let idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("DELETE_GLOBAL requires argument".into())
                })?;
                let name = self.names.get(idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", idx))
                })?;
                let _ = self.globals.delitem(&PyKey::Str(Arc::from(name.as_str())));
            }
            Opcode::DeleteAttr | Opcode::DeleteSubscr => {
                // TODO: Implement delete operations
                return Err(InterpreterError::Runtime(
                    "DELETE operations not fully implemented".into(),
                ));
            }

            // Binary operations
            Opcode::BinaryAdd => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_add(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinarySub => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_sub(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryMul => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_mul(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryDiv => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_div(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryFloorDiv => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_floordiv(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryMod => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_mod(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryPow => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_pow(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryAnd => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_and(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryOr => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_or(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryXor => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_xor(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryLshift => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_lshift(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryRshift => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_rshift(&a, &b)?;
                frame.push(result);
            }
            Opcode::BinaryMatMul => {
                return Err(InterpreterError::Runtime("Matrix multiply not implemented".into()));
            }

            // Unary operations
            Opcode::UnaryNot => {
                let a = frame.pop();
                frame.push(PyValue::Bool(!a.to_bool()));
            }
            Opcode::UnaryNeg => {
                let a = frame.pop();
                let result = self.unary_neg(&a)?;
                frame.push(result);
            }
            Opcode::UnaryPos => {
                let a = frame.pop();
                let result = self.unary_pos(&a)?;
                frame.push(result);
            }
            Opcode::UnaryInvert => {
                let a = frame.pop();
                let result = self.unary_invert(&a)?;
                frame.push(result);
            }

            // In-place operations (same as binary for interpreter)
            Opcode::InplaceAdd => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_add(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceSub => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_sub(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceMul => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_mul(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceDiv => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_div(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceFloorDiv => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_floordiv(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceMod => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_mod(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplacePow => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_pow(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceAnd => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_and(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceOr => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_or(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceXor => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_xor(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceLshift => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_lshift(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceRshift => {
                let b = frame.pop();
                let a = frame.pop();
                let result = self.binary_rshift(&a, &b)?;
                frame.push(result);
            }
            Opcode::InplaceMatMul => {
                return Err(InterpreterError::Runtime(
                    "In-place matrix multiply not implemented".into(),
                ));
            }

            // Comparison operations
            Opcode::CompareEq => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(self.compare_eq(&a, &b)));
            }
            Opcode::CompareNe => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(!self.compare_eq(&a, &b)));
            }
            Opcode::CompareLt => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(self.compare_lt(&a, &b)?));
            }
            Opcode::CompareLe => {
                let b = frame.pop();
                let a = frame.pop();
                let lt = self.compare_lt(&a, &b)?;
                let eq = self.compare_eq(&a, &b);
                frame.push(PyValue::Bool(lt || eq));
            }
            Opcode::CompareGt => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(self.compare_lt(&b, &a)?));
            }
            Opcode::CompareGe => {
                let b = frame.pop();
                let a = frame.pop();
                let gt = self.compare_lt(&b, &a)?;
                let eq = self.compare_eq(&a, &b);
                frame.push(PyValue::Bool(gt || eq));
            }
            Opcode::CompareIs => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(self.compare_is(&a, &b)));
            }
            Opcode::CompareIsNot => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(!self.compare_is(&a, &b)));
            }
            Opcode::CompareIn => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(self.compare_in(&a, &b)?));
            }
            Opcode::CompareNotIn => {
                let b = frame.pop();
                let a = frame.pop();
                frame.push(PyValue::Bool(!self.compare_in(&a, &b)?));
            }
            Opcode::ExceptionMatch => {
                let exc_type = frame.pop();
                let exc = frame.peek();
                let matches = match (exc, &exc_type) {
                    (PyValue::Exception(e), PyValue::Str(type_name)) => e.is_instance(type_name),
                    _ => false,
                };
                frame.push(PyValue::Bool(matches));
            }

            // Control flow
            Opcode::Jump => {
                let offset =
                    arg.ok_or_else(|| InterpreterError::Runtime("JUMP requires argument".into()))?;
                // Interpret as signed relative offset
                let offset = offset as i16;
                frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
            }
            Opcode::JumpIfTrue => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("JUMP_IF_TRUE requires argument".into())
                })?;
                if frame.peek().to_bool() {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                }
            }
            Opcode::JumpIfFalse => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("JUMP_IF_FALSE requires argument".into())
                })?;
                if !frame.peek().to_bool() {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                }
            }
            Opcode::JumpIfTrueOrPop => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("JUMP_IF_TRUE_OR_POP requires argument".into())
                })?;
                if frame.peek().to_bool() {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                } else {
                    frame.pop();
                }
            }
            Opcode::JumpIfFalseOrPop => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("JUMP_IF_FALSE_OR_POP requires argument".into())
                })?;
                if !frame.peek().to_bool() {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                } else {
                    frame.pop();
                }
            }
            Opcode::PopJumpIfTrue => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("POP_JUMP_IF_TRUE requires argument".into())
                })?;
                let value = frame.pop();
                if value.to_bool() {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                }
            }
            Opcode::PopJumpIfFalse => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("POP_JUMP_IF_FALSE requires argument".into())
                })?;
                let value = frame.pop();
                if !value.to_bool() {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                }
            }
            Opcode::PopJumpIfNone => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("POP_JUMP_IF_NONE requires argument".into())
                })?;
                let value = frame.pop();
                if matches!(value, PyValue::None) {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                }
            }
            Opcode::PopJumpIfNotNone => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("POP_JUMP_IF_NOT_NONE requires argument".into())
                })?;
                let value = frame.pop();
                if !matches!(value, PyValue::None) {
                    let offset = offset as i16;
                    frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                }
            }
            Opcode::Return => {
                let value = frame.pop();
                return Ok(DispatchResult::Return(value));
            }

            // Iteration
            Opcode::GetIter => {
                let obj = frame.pop();
                let iter = match obj {
                    PyValue::List(list) => {
                        PyValue::Iterator(Arc::new(dx_py_core::PyIterator::new(list.to_vec())))
                    }
                    PyValue::Set(set) => {
                        PyValue::Iterator(Arc::new(dx_py_core::PyIterator::new(set.to_vec())))
                    }
                    PyValue::Tuple(tuple) => {
                        PyValue::Iterator(Arc::new(dx_py_core::PyIterator::new(tuple.to_vec())))
                    }
                    PyValue::Str(s) => {
                        let chars: Vec<PyValue> =
                            s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect();
                        PyValue::Iterator(Arc::new(dx_py_core::PyIterator::new(chars)))
                    }
                    PyValue::Iterator(_) => obj,
                    // Generators are their own iterators
                    PyValue::Generator(_) => obj,
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object is not iterable",
                            obj.type_name()
                        )));
                    }
                };
                frame.push(iter);
            }
            Opcode::ForIter => {
                let offset = arg.ok_or_else(|| {
                    InterpreterError::Runtime("FOR_ITER requires argument".into())
                })?;

                let iter = frame.peek().clone();
                match iter {
                    PyValue::Iterator(it) => {
                        if let Some(value) = it.next() {
                            frame.push(value);
                        } else {
                            frame.pop();
                            // Jump to end of loop (relative offset)
                            let offset = offset as i16;
                            frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                        }
                    }
                    PyValue::Generator(gen) => {
                        // Execute the generator to get the next value
                        match self.execute_generator_next(&gen)? {
                            Some(value) => {
                                frame.push(value);
                            }
                            None => {
                                // Generator exhausted
                                frame.pop();
                                let offset = offset as i16;
                                frame.ip = ((frame.ip as i32) + (offset as i32)) as usize;
                            }
                        }
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object is not an iterator",
                            iter.type_name()
                        )));
                    }
                }
            }

            // Generator operations
            Opcode::Yield => {
                let value = frame.pop();
                return Ok(DispatchResult::Yield(value));
            }
            Opcode::YieldFrom => {
                // YIELD_FROM implements delegation to a sub-iterator or coroutine
                // Stack: [sub_iterator/coroutine] -> yields values from sub_iterator/coroutine
                //
                // The sub-iterator/coroutine is on top of the stack (already converted by GET_ITER/GET_AWAITABLE).
                // We:
                // 1. Call next() on the sub-iterator or send(None) on the coroutine
                // 2. If it yields a value, yield that value (keeping sub_iterator/coroutine on stack)
                // 3. If it raises StopIteration, pop the sub_iterator/coroutine and continue
                //
                // The arg is not used for now (could be used for send/throw propagation)
                let _target = arg.unwrap_or(0);
                
                let sub_iter = frame.peek().clone();
                
                match &sub_iter {
                    PyValue::Generator(gen) => {
                        // For generators, use the generator execution machinery
                        use dx_py_core::pygenerator::GeneratorResult;
                        
                        // Check if generator needs execution or already has a result
                        let result = gen.next();
                        
                        match result {
                            GeneratorResult::NeedExecution => {
                                // Execute the sub-generator
                                match self.execute_generator_next(gen) {
                                    Ok(Some(value)) => {
                                        // Sub-generator yielded - yield this value
                                        // Keep sub_iter on stack for next iteration
                                        // Decrement IP so we re-execute YIELD_FROM on resume
                                        frame.ip -= 3; // Go back to YIELD_FROM (1 byte opcode + 2 byte arg)
                                        return Ok(DispatchResult::Yield(value));
                                    }
                                    Ok(None) => {
                                        // Sub-generator exhausted (StopIteration)
                                        // Pop the sub-iterator and push None as the result
                                        frame.pop(); // Remove sub_iter
                                        frame.push(PyValue::None);
                                        // Continue execution (don't yield)
                                    }
                                    Err(e) => {
                                        // Sub-generator raised an exception
                                        frame.pop(); // Remove sub_iter
                                        return Err(e);
                                    }
                                }
                            }
                            GeneratorResult::Yielded(value) => {
                                // Already have a yielded value (shouldn't happen normally)
                                frame.ip -= 3;
                                return Ok(DispatchResult::Yield(value));
                            }
                            GeneratorResult::StopIteration(return_value) => {
                                // Sub-generator is done
                                frame.pop(); // Remove sub_iter
                                frame.push(return_value);
                                // Continue execution
                            }
                            GeneratorResult::Error(msg) => {
                                frame.pop();
                                return Err(InterpreterError::Runtime(msg));
                            }
                            GeneratorResult::Closed => {
                                frame.pop();
                                frame.push(PyValue::None);
                            }
                        }
                    }
                    PyValue::Coroutine(coro) => {
                        // For coroutines, use the coroutine execution machinery
                        use dx_py_core::pygenerator::CoroutineResult;
                        
                        // Check if coroutine needs execution
                        let result = coro.send(PyValue::None);
                        
                        match result {
                            CoroutineResult::NeedExecution => {
                                // Execute the coroutine
                                match self.execute_coroutine_next(coro) {
                                    Ok(Some(value)) => {
                                        // Coroutine yielded (awaiting) - yield this value
                                        // Keep coroutine on stack for next iteration
                                        frame.ip -= 3; // Go back to YIELD_FROM
                                        return Ok(DispatchResult::Yield(value));
                                    }
                                    Ok(None) => {
                                        // Coroutine completed
                                        // Pop the coroutine and push None as the result
                                        frame.pop(); // Remove coroutine
                                        frame.push(PyValue::None);
                                        // Continue execution
                                    }
                                    Err(e) => {
                                        // Coroutine raised an exception
                                        frame.pop(); // Remove coroutine
                                        return Err(e);
                                    }
                                }
                            }
                            CoroutineResult::Awaiting(value) => {
                                // Already have an awaiting value
                                frame.ip -= 3;
                                return Ok(DispatchResult::Yield(value));
                            }
                            CoroutineResult::StopIteration(return_value) => {
                                // Coroutine is done
                                frame.pop(); // Remove coroutine
                                frame.push(return_value);
                                // Continue execution
                            }
                            CoroutineResult::Error(msg) => {
                                frame.pop();
                                return Err(InterpreterError::Runtime(msg));
                            }
                            CoroutineResult::Closed => {
                                frame.pop();
                                frame.push(PyValue::None);
                            }
                        }
                    }
                    PyValue::Iterator(iter) => {
                        // For regular iterators, get the next value
                        if let Some(value) = iter.next() {
                            // Iterator yielded - yield this value
                            // Keep sub_iter on stack for next iteration
                            frame.ip -= 3; // Go back to YIELD_FROM
                            return Ok(DispatchResult::Yield(value));
                        } else {
                            // Iterator exhausted
                            frame.pop(); // Remove sub_iter
                            frame.push(PyValue::None);
                            // Continue execution
                        }
                    }
                    _ => {
                        // For other types, try to treat them as iterables
                        // This shouldn't normally happen since GET_ITER is emitted before YIELD_FROM
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object is not an iterator",
                            sub_iter.type_name()
                        )));
                    }
                }
            }
            Opcode::GetAwaitable => {
                // GET_AWAITABLE checks if an object is awaitable and returns it
                // For now, coroutines are awaitable. In the future, we can check for __await__ method
                let obj = frame.pop();
                match &obj {
                    PyValue::Coroutine(_) => {
                        // Coroutines are awaitable
                        frame.push(obj);
                    }
                    PyValue::Generator(_) => {
                        // Generators can be awaitable if they're async generators
                        // For now, treat them as awaitable
                        frame.push(obj);
                    }
                    _ => {
                        // Check if the object has an __await__ method
                        // For now, just push the object and let YieldFrom handle it
                        frame.push(obj);
                    }
                }
            }
            Opcode::GetAiter => {
                // GET_AITER gets an async iterator from an object
                // It calls __aiter__() on the object
                let obj = frame.pop();
                
                // For now, we'll create a simple async iterator wrapper
                // In a full implementation, we would call __aiter__() method
                match &obj {
                    PyValue::List(list) => {
                        // Convert list to async iterator
                        let items = list.to_vec();
                        frame.push(PyValue::Iterator(Arc::new(PyIterator::new(items))));
                    }
                    PyValue::Tuple(tuple) => {
                        // Convert tuple to async iterator
                        let items = tuple.to_vec();
                        frame.push(PyValue::Iterator(Arc::new(PyIterator::new(items))));
                    }
                    PyValue::Iterator(_) => {
                        // Already an iterator
                        frame.push(obj);
                    }
                    _ => {
                        // For other types, try to treat as iterable
                        // In a full implementation, we would call __aiter__()
                        frame.push(obj);
                    }
                }
            }
            Opcode::GetAnext => {
                // GET_ANEXT gets the next value from an async iterator
                // It calls __anext__() on the async iterator
                // This returns an awaitable that yields the next value
                let async_iter = frame.peek().clone();
                
                match &async_iter {
                    PyValue::Iterator(iter) => {
                        // Get next value from iterator
                        if let Some(value) = iter.next() {
                            // Create a simple "awaitable" that yields this value
                            // In a full implementation, __anext__() returns a coroutine
                            frame.push(value);
                        } else {
                            // Iterator exhausted - raise StopAsyncIteration
                            // For now, push None
                            frame.push(PyValue::None);
                        }
                    }
                    _ => {
                        // Not an async iterator
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object is not an async iterator",
                            async_iter.type_name()
                        )));
                    }
                }
            }
            Opcode::EndAsyncFor => {
                // END_ASYNC_FOR: Handle end of async for loop
                // This is similar to handling StopAsyncIteration
                // For now, just a placeholder
                let _target = arg.unwrap_or(0);
                // Pop the async iterator
                frame.pop();
            }
            Opcode::Send => {
                // SEND: Send a value to a coroutine/generator
                // Stack: [coroutine, value] -> [result]
                let _target = arg.unwrap_or(0);
                let value = frame.pop();
                let coro_or_gen = frame.pop();
                
                match coro_or_gen {
                    PyValue::Coroutine(coro) => {
                        // Send value to coroutine
                        use dx_py_core::pygenerator::CoroutineResult;
                        let result = coro.send(value);
                        match result {
                            CoroutineResult::NeedExecution => {
                                // Execute the coroutine
                                match self.execute_coroutine_next(&coro) {
                                    Ok(Some(yielded)) => frame.push(yielded),
                                    Ok(None) => frame.push(PyValue::None),
                                    Err(e) => return Err(e),
                                }
                            }
                            CoroutineResult::Awaiting(v) => frame.push(v),
                            CoroutineResult::StopIteration(v) => frame.push(v),
                            CoroutineResult::Error(msg) => {
                                return Err(InterpreterError::Runtime(msg))
                            }
                            CoroutineResult::Closed => frame.push(PyValue::None),
                        }
                    }
                    PyValue::Generator(gen) => {
                        // Send value to generator
                        use dx_py_core::pygenerator::GeneratorResult;
                        let result = gen.send(value);
                        match result {
                            GeneratorResult::NeedExecution => {
                                match self.execute_generator_next(&gen) {
                                    Ok(Some(yielded)) => frame.push(yielded),
                                    Ok(None) => frame.push(PyValue::None),
                                    Err(e) => return Err(e),
                                }
                            }
                            GeneratorResult::Yielded(v) => frame.push(v),
                            GeneratorResult::StopIteration(v) => frame.push(v),
                            GeneratorResult::Error(msg) => {
                                return Err(InterpreterError::Runtime(msg))
                            }
                            GeneratorResult::Closed => frame.push(PyValue::None),
                        }
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "can't send to '{}'",
                            coro_or_gen.type_name()
                        )));
                    }
                }
            }

            // Function calls
            Opcode::Call => {
                let argc =
                    arg.ok_or_else(|| InterpreterError::Runtime("CALL requires argument".into()))?;

                let mut args = Vec::with_capacity(argc);
                for _ in 0..argc {
                    args.push(frame.pop());
                }
                args.reverse();

                let callable = frame.pop();

                let result = match callable {
                    PyValue::Builtin(builtin) => {
                        // Special handling for __build_class__
                        if builtin.name == "__build_class__" {
                            self.handle_build_class(&args)?
                        } else if builtin.name == "next" {
                            // Special handling for next() on generators
                            self.handle_builtin_next(&args)?
                        } else {
                            builtin.call(&args).map_err(InterpreterError::RuntimeError)?
                        }
                    }
                    PyValue::Function(func) => {
                        // Call user-defined function
                        self.call_user_function(&func, &args)?
                    }
                    PyValue::BoundMethod(ref method) => {
                        // Handle bound method call
                        self.call_bound_method(method, &args)?
                    }
                    PyValue::Type(ref class) => {
                        // Class instantiation - create a new instance
                        self.instantiate_class(class, &args)?
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object is not callable",
                            callable.type_name()
                        )));
                    }
                };

                frame.push(result);
            }
            Opcode::CallMethod => {
                // CALL_METHOD is an optimization for method calls
                // Works in conjunction with LOAD_METHOD
                //
                // Stack layout after LOAD_METHOD + pushing args:
                //   Case 1 (unbound method): [method, self, arg0, ..., argN]
                //   Case 2 (bound/other):    [NULL, callable, arg0, ..., argN]
                //
                // The NULL marker (PyValue::None from LOAD_METHOD) tells us which case we're in

                let argc = arg.ok_or_else(|| {
                    InterpreterError::Runtime("CALL_METHOD requires argument".into())
                })?;

                // Pop arguments
                let mut args = Vec::with_capacity(argc);
                for _ in 0..argc {
                    args.push(frame.pop());
                }
                args.reverse();

                // Pop self_or_callable (this is either 'self' or the callable depending on case)
                let self_or_callable = frame.pop();
                // Pop method_or_null (this is either the method or NULL marker)
                let method_or_null = frame.pop();

                let result = if matches!(method_or_null, PyValue::None) {
                    // Case 2: NULL marker means self_or_callable is the actual callable
                    // (bound method or other callable)
                    match self_or_callable {
                        PyValue::Builtin(builtin) => {
                            builtin.call(&args).map_err(InterpreterError::RuntimeError)?
                        }
                        PyValue::Function(func) => self.call_user_function(&func, &args)?,
                        PyValue::BoundMethod(ref method) => {
                            self.call_bound_method(method, &args)?
                        }
                        PyValue::Type(ref class) => self.instantiate_class(class, &args)?,
                        _ => {
                            return Err(InterpreterError::TypeError(format!(
                                "'{}' object is not callable",
                                self_or_callable.type_name()
                            )));
                        }
                    }
                } else {
                    // Case 1: method_or_null is the actual method, self_or_callable is 'self'
                    // Prepend self to args and call the method
                    let mut full_args = vec![self_or_callable];
                    full_args.extend(args);

                    match method_or_null {
                        PyValue::Function(func) => self.call_user_function(&func, &full_args)?,
                        PyValue::Builtin(builtin) => {
                            builtin.call(&full_args).map_err(InterpreterError::RuntimeError)?
                        }
                        PyValue::BoundMethod(ref method) => {
                            // This shouldn't normally happen, but handle it gracefully
                            self.call_bound_method(method, &full_args)?
                        }
                        _ => {
                            return Err(InterpreterError::TypeError(format!(
                                "'{}' object is not callable",
                                method_or_null.type_name()
                            )));
                        }
                    }
                };

                frame.push(result);
            }
            Opcode::CallKw | Opcode::CallEx => {
                return Err(InterpreterError::Runtime(
                    "Advanced call opcodes not implemented".into(),
                ));
            }
            Opcode::MakeFunction => {
                let flags = arg.ok_or_else(|| {
                    InterpreterError::Runtime("MAKE_FUNCTION requires argument".into())
                })?;

                // Stack order: [qualname, code_obj] with code_obj on top (Python bytecode order)
                // Pop code object first (it's on top)
                let code_obj = frame.pop();
                // Pop qualified name
                let qualname = frame.pop();

                // Extract PyCode from PyValue
                let code = match &code_obj {
                    PyValue::Code(c) => c,
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "expected code object, got {}",
                            code_obj.type_name()
                        )))
                    }
                };

                // Extract qualname string
                let qualname_str = match &qualname {
                    PyValue::Str(s) => s.to_string(),
                    _ => code.name.to_string(),
                };

                // Build parameters from code object's varnames
                let mut params = Vec::new();
                for i in 0..code.argcount as usize {
                    if let Some(name) = code.varnames.get(i) {
                        params.push(dx_py_core::pyfunction::Parameter {
                            name: name.to_string(),
                            kind: if i < code.posonlyargcount as usize {
                                dx_py_core::pyfunction::ParameterKind::Positional
                            } else {
                                dx_py_core::pyfunction::ParameterKind::PositionalOrKeyword
                            },
                            default: None,
                            annotation: None,
                        });
                    }
                }

                // Add keyword-only parameters
                let kwonly_start = code.argcount as usize;
                for i in 0..code.kwonlyargcount as usize {
                    if let Some(name) = code.varnames.get(kwonly_start + i) {
                        params.push(dx_py_core::pyfunction::Parameter {
                            name: name.to_string(),
                            kind: dx_py_core::pyfunction::ParameterKind::KeywordOnly,
                            default: None,
                            annotation: None,
                        });
                    }
                }

                // Create CodeRef
                let code_ref = dx_py_core::pyfunction::CodeRef {
                    bytecode_offset: 0,
                    num_locals: code.nlocals as u16,
                    stack_size: code.stacksize as u16,
                    num_args: code.argcount as u8,
                    num_kwonly_args: code.kwonlyargcount as u8,
                };

                // Create PyFunction
                let mut func =
                    dx_py_core::pyfunction::PyFunction::new(qualname_str.clone(), code_ref, params);

                // Set function flags based on code flags
                func.flags.has_varargs = code.has_varargs();
                func.flags.has_kwargs = code.has_varkeywords();
                func.flags.is_generator = code.is_generator();
                func.flags.is_coroutine = code.is_coroutine();

                // Handle flags for defaults, closure, annotations
                // Flag 0x01: has positional defaults tuple
                if flags & 0x01 != 0 {
                    let defaults = frame.pop();
                    if let PyValue::Tuple(t) = defaults {
                        func = func.with_defaults(t.to_vec());
                    }
                }

                // Flag 0x02: has keyword-only defaults dict
                if flags & 0x02 != 0 {
                    let _kwdefaults = frame.pop();
                    // TODO: Handle keyword-only defaults
                }

                // Flag 0x04: has annotations dict
                if flags & 0x04 != 0 {
                    let _annotations = frame.pop();
                    // TODO: Handle annotations
                }

                // Flag 0x08: has closure tuple
                if flags & 0x08 != 0 {
                    let closure = frame.pop();
                    if let PyValue::Tuple(t) = closure {
                        // Store the closure cells, prepending the code object for lookup
                        let mut closure_with_code = vec![code_obj.clone()];
                        closure_with_code.extend(t.to_vec());
                        func = func.with_closure(closure_with_code);
                    }
                } else {
                    // No closure provided, just store the code object
                    func = func.with_closure(vec![code_obj.clone()]);
                }

                frame.push(PyValue::Function(Arc::new(func)));
            }
            Opcode::MakeClosure => {
                // MakeClosure is similar to MakeFunction but always has closure cells
                let flags = arg.ok_or_else(|| {
                    InterpreterError::Runtime("MAKE_CLOSURE requires argument".into())
                })?;

                // Pop closure tuple first (always present for MakeClosure)
                let closure = frame.pop();

                // Pop code object from stack
                let code_obj = frame.pop();
                // Pop qualified name
                let qualname = frame.pop();

                // Extract PyCode from PyValue
                let code = match &code_obj {
                    PyValue::Code(c) => c,
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "expected code object, got {}",
                            code_obj.type_name()
                        )))
                    }
                };

                // Extract qualname string
                let qualname_str = match &qualname {
                    PyValue::Str(s) => s.to_string(),
                    _ => code.name.to_string(),
                };

                // Build parameters from code object's varnames
                let mut params = Vec::new();
                for i in 0..code.argcount as usize {
                    if let Some(name) = code.varnames.get(i) {
                        params.push(dx_py_core::pyfunction::Parameter {
                            name: name.to_string(),
                            kind: if i < code.posonlyargcount as usize {
                                dx_py_core::pyfunction::ParameterKind::Positional
                            } else {
                                dx_py_core::pyfunction::ParameterKind::PositionalOrKeyword
                            },
                            default: None,
                            annotation: None,
                        });
                    }
                }

                // Add keyword-only parameters
                let kwonly_start = code.argcount as usize;
                for i in 0..code.kwonlyargcount as usize {
                    if let Some(name) = code.varnames.get(kwonly_start + i) {
                        params.push(dx_py_core::pyfunction::Parameter {
                            name: name.to_string(),
                            kind: dx_py_core::pyfunction::ParameterKind::KeywordOnly,
                            default: None,
                            annotation: None,
                        });
                    }
                }

                // Create CodeRef
                let code_ref = dx_py_core::pyfunction::CodeRef {
                    bytecode_offset: 0,
                    num_locals: code.nlocals as u16,
                    stack_size: code.stacksize as u16,
                    num_args: code.argcount as u8,
                    num_kwonly_args: code.kwonlyargcount as u8,
                };

                // Create PyFunction with closure
                let mut func =
                    dx_py_core::pyfunction::PyFunction::new(qualname_str.clone(), code_ref, params);

                // Set function flags
                func.flags.has_varargs = code.has_varargs();
                func.flags.has_kwargs = code.has_varkeywords();
                func.flags.is_generator = code.is_generator();
                func.flags.is_coroutine = code.is_coroutine();

                // Set closure
                if let PyValue::Tuple(t) = closure {
                    func = func.with_closure(t.to_vec());
                }

                // Handle additional flags for defaults and annotations
                // Flag 0x01: has positional defaults tuple
                if flags & 0x01 != 0 {
                    let defaults = frame.pop();
                    if let PyValue::Tuple(t) = defaults {
                        func = func.with_defaults(t.to_vec());
                    }
                }

                // Flag 0x02: has keyword-only defaults dict
                if flags & 0x02 != 0 {
                    let _kwdefaults = frame.pop();
                    // TODO: Handle keyword-only defaults
                }

                // Flag 0x04: has annotations dict
                if flags & 0x04 != 0 {
                    let _annotations = frame.pop();
                    // TODO: Handle annotations
                }

                frame.push(PyValue::Function(Arc::new(func)));
            }
            Opcode::PushNull => {
                // Push a null/None value onto the stack for method calls
                frame.push(PyValue::None);
            }
            Opcode::KwNames => {
                // KwNames stores keyword argument names - just skip for now
                let _idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("KW_NAMES requires argument".into())
                })?;
            }

            // Object creation
            Opcode::BuildTuple => {
                let count = arg.ok_or_else(|| {
                    InterpreterError::Runtime("BUILD_TUPLE requires argument".into())
                })?;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(frame.pop());
                }
                items.reverse();
                frame.push(PyValue::Tuple(Arc::new(dx_py_core::PyTuple::from_values(items))));
            }
            Opcode::BuildList => {
                let count = arg.ok_or_else(|| {
                    InterpreterError::Runtime("BUILD_LIST requires argument".into())
                })?;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(frame.pop());
                }
                items.reverse();
                frame.push(PyValue::List(Arc::new(dx_py_core::PyList::from_values(items))));
            }
            Opcode::BuildSet => {
                let count = arg.ok_or_else(|| {
                    InterpreterError::Runtime("BUILD_SET requires argument".into())
                })?;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(frame.pop());
                }
                items.reverse();
                frame.push(PyValue::Set(Arc::new(dx_py_core::PySet::from_values(items))));
            }
            Opcode::BuildDict => {
                let count = arg.ok_or_else(|| {
                    InterpreterError::Runtime("BUILD_DICT requires argument".into())
                })?;
                let dict = Arc::new(dx_py_core::PyDict::new());
                let mut pairs = Vec::with_capacity(count);
                for _ in 0..count {
                    let value = frame.pop();
                    let key = frame.pop();
                    pairs.push((key, value));
                }
                pairs.reverse();
                for (key, value) in pairs {
                    let py_key = match key {
                        PyValue::Str(s) => PyKey::Str(s),
                        PyValue::Int(i) => PyKey::Int(i),
                        _ => return Err(InterpreterError::TypeError("unhashable type".into())),
                    };
                    dict.setitem(py_key, value);
                }
                frame.push(PyValue::Dict(dict));
            }
            Opcode::BuildString => {
                let count = arg.ok_or_else(|| {
                    InterpreterError::Runtime("BUILD_STRING requires argument".into())
                })?;
                let mut parts = Vec::with_capacity(count);
                for _ in 0..count {
                    parts.push(frame.pop());
                }
                parts.reverse();
                let result: String = parts.iter().map(|v| format!("{:?}", v)).collect();
                frame.push(PyValue::Str(Arc::from(result)));
            }
            Opcode::BuildSlice => {
                // TODO: Implement slice objects
                return Err(InterpreterError::Runtime("BUILD_SLICE not implemented".into()));
            }
            Opcode::ListAppend => {
                // LIST_APPEND(i) - Calls list.append(TOS[-i], TOS)
                // The depth argument specifies how deep in the stack the list is
                // For list comprehensions, depth=2 means: TOS=element, TOS-1=iterator, TOS-2=list
                let depth = arg.unwrap_or(1);
                if depth == 0 {
                    return Err(InterpreterError::Runtime("LIST_APPEND depth must be > 0".into()));
                }
                let value = frame.pop();
                // peek_n(n) looks at position stack.len() - n - 1, so for depth=2:
                // we want the item at stack.len() - 2, which is peek_n(1)
                let list = frame.peek_n(depth - 1).clone();
                if let PyValue::List(l) = list {
                    l.append(value);
                } else {
                    return Err(InterpreterError::TypeError(format!(
                        "LIST_APPEND expected list at depth {}, got {}",
                        depth,
                        list.type_name()
                    )));
                }
            }
            Opcode::SetAdd => {
                // SET_ADD(i) - Calls set.add(TOS[-i], TOS)
                // Similar to LIST_APPEND but for sets
                let depth = arg.unwrap_or(1);
                if depth == 0 {
                    return Err(InterpreterError::Runtime("SET_ADD depth must be > 0".into()));
                }
                let value = frame.pop();
                let set = frame.peek_n(depth - 1).clone();
                if let PyValue::Set(s) = set {
                    s.add(value);
                } else {
                    return Err(InterpreterError::TypeError(format!(
                        "SET_ADD expected set at depth {}, got {}",
                        depth,
                        set.type_name()
                    )));
                }
            }
            Opcode::MapAdd => {
                // MAP_ADD(i) - Calls dict.__setitem__(TOS1[-i], TOS1, TOS)
                // The depth argument specifies how deep in the stack the dict is
                // For dict comprehensions: TOS=value, TOS-1=key, TOS-2=iterator, TOS-3=dict
                let depth = arg.unwrap_or(1);
                if depth < 2 {
                    return Err(InterpreterError::Runtime("MAP_ADD depth must be >= 2".into()));
                }
                let value = frame.pop();
                let key = frame.pop();
                // After popping value and key, the dict is at depth-2 from the new top
                let dict = frame.peek_n(depth - 2).clone();
                if let PyValue::Dict(d) = dict {
                    let py_key = match key {
                        PyValue::Str(s) => PyKey::Str(s),
                        PyValue::Int(i) => PyKey::Int(i),
                        _ => return Err(InterpreterError::TypeError("unhashable type".into())),
                    };
                    d.setitem(py_key, value);
                } else {
                    return Err(InterpreterError::TypeError(format!(
                        "MAP_ADD expected dict at depth {}, got {}",
                        depth,
                        dict.type_name()
                    )));
                }
            }
            Opcode::UnpackSequence => {
                let count = arg.ok_or_else(|| {
                    InterpreterError::Runtime("UNPACK_SEQUENCE requires argument".into())
                })?;
                let seq = frame.pop();
                let items: Vec<PyValue> = match seq {
                    PyValue::List(l) => l.to_vec(),
                    PyValue::Tuple(t) => t.to_vec(),
                    _ => {
                        return Err(InterpreterError::TypeError(
                            "cannot unpack non-sequence".into(),
                        ))
                    }
                };
                if items.len() != count {
                    return Err(InterpreterError::ValueError(format!(
                        "not enough values to unpack (expected {}, got {})",
                        count,
                        items.len()
                    )));
                }
                for item in items.into_iter().rev() {
                    frame.push(item);
                }
            }
            Opcode::BuildClass => {
                // BUILD_CLASS creates a class from:
                // Stack: class_body_func, class_name, *bases -> class
                // The argument specifies the number of base classes
                let num_bases = arg.unwrap_or(0);

                // Pop base classes (in reverse order)
                let mut bases: Vec<Arc<dx_py_core::types::PyType>> = Vec::with_capacity(num_bases);
                for _ in 0..num_bases {
                    let base = frame.pop();
                    match base {
                        PyValue::Type(t) => bases.push(t),
                        _ => {
                            return Err(InterpreterError::TypeError(format!(
                                "bases must be types, not '{}'",
                                base.type_name()
                            )))
                        }
                    }
                }
                bases.reverse(); // Restore original order

                // Pop class name
                let name = frame.pop();
                let class_name = match &name {
                    PyValue::Str(s) => s.to_string(),
                    _ => {
                        return Err(InterpreterError::TypeError(
                            "class name must be a string".into(),
                        ))
                    }
                };

                // Pop class body function
                let body_func = frame.pop();

                // Create the class object with computed MRO
                let class = dx_py_core::types::PyType::with_bases(class_name.clone(), bases);

                // Execute class body function to populate namespace
                // The class body function takes __locals__ dict as argument
                // and populates it with class attributes/methods
                if let PyValue::Function(func) = &body_func {
                    // Execute the class body to populate the namespace
                    // For now, we call the function with an empty args list
                    // The function should populate the class dict via STORE_NAME
                    match self.call_user_function(func, &[]) {
                        Ok(PyValue::Dict(d)) => {
                            // If the body returned a dict, merge it into class dict
                            for (key, value) in d.items() {
                                let key_str = match &key {
                                    PyKey::Str(s) => s.to_string(),
                                    PyKey::Int(i) => i.to_string(),
                                    PyKey::Bool(b) => b.to_string(),
                                    PyKey::None => "None".to_string(),
                                    PyKey::Tuple(_) => format!("{:?}", key),
                                };
                                class.dict.insert(key_str, value);
                            }
                        }
                        Ok(_) => {
                            // Body didn't return a dict, that's fine
                            // The class body may have populated via STORE_NAME
                        }
                        Err(_) => {
                            // Ignore errors from class body execution for now
                            // The class is still created
                        }
                    }
                } else if let PyValue::Dict(d) = &body_func {
                    // If we got a dict directly (pre-populated namespace), use it
                    for (key, value) in d.items() {
                        let key_str = match &key {
                            PyKey::Str(s) => s.to_string(),
                            PyKey::Int(i) => i.to_string(),
                            PyKey::Bool(b) => b.to_string(),
                            PyKey::None => "None".to_string(),
                            PyKey::Tuple(_) => format!("{:?}", key),
                        };
                        class.dict.insert(key_str, value);
                    }
                }

                // Push the class onto the stack
                frame.push(PyValue::Type(Arc::new(class)));
            }

            // Exception handling
            Opcode::SetupExcept => {
                let handler = arg.ok_or_else(|| {
                    InterpreterError::Runtime("SETUP_EXCEPT requires argument".into())
                })?;
                frame.push_block(dx_py_core::pyframe::BlockType::Except, handler);
            }
            Opcode::SetupFinally => {
                let handler = arg.ok_or_else(|| {
                    InterpreterError::Runtime("SETUP_FINALLY requires argument".into())
                })?;
                frame.push_block(dx_py_core::pyframe::BlockType::Finally, handler);
            }
            Opcode::PopExcept => {
                frame.pop_block();
            }
            Opcode::EndFinally => {
                // END_FINALLY: End of a finally block
                // The stack contains a marker indicating why we entered the finally block:
                // - Int(0) or None = normal exit (continue execution)
                // - Exception = exceptional exit (re-raise it)
                // - Int(2) = return pending (return value is below marker on stack)
                // - Int(3) = break pending
                // - Int(4) = continue pending
                let marker = frame.pop();
                match &marker {
                    PyValue::Exception(_) => {
                        // Re-raise the exception after finally block completes
                        return Ok(DispatchResult::Exception(marker));
                    }
                    PyValue::Int(2) => {
                        // Return pending - pop the return value and return it
                        let return_value = frame.pop();
                        return Ok(DispatchResult::Return(return_value));
                    }
                    PyValue::Int(3) => {
                        // Break pending - find the loop block and break out
                        // For now, we'll handle this by continuing execution
                        // The break should have been handled before entering finally
                    }
                    PyValue::Int(4) => {
                        // Continue pending - find the loop block and continue
                        // For now, we'll handle this by continuing execution
                        // The continue should have been handled before entering finally
                    }
                    PyValue::None | PyValue::Int(0) => {
                        // Normal exit from finally block - continue execution
                    }
                    _ => {
                        // Unknown marker - treat as normal exit
                        // This handles legacy code that might push other values
                    }
                }
            }
            Opcode::Raise => {
                let argc = arg.unwrap_or(0);
                let exc = match argc {
                    0 => {
                        // Bare raise - re-raise the current exception
                        // This is only valid inside an exception handler
                        if frame.in_exception_handler() {
                            // Look for the exception on the stack
                            // The exception should be at the top of the stack in an except handler
                            // We search from top to bottom to find the first exception
                            let mut found_exc = None;
                            for i in (0..frame.stack.len()).rev() {
                                if let PyValue::Exception(_) = &frame.stack[i] {
                                    found_exc = Some(frame.stack[i].clone());
                                    break;
                                }
                            }
                            
                            if let Some(exc) = found_exc {
                                exc
                            } else {
                                return Err(InterpreterError::Runtime(
                                    "No active exception to re-raise".into(),
                                ));
                            }
                        } else {
                            return Err(InterpreterError::Runtime(
                                "No active exception to re-raise".into(),
                            ));
                        }
                    }
                    1 => {
                        // raise exc
                        let exc_value = frame.pop();
                        // If it's a Type, instantiate it with no args
                        match exc_value {
                            PyValue::Type(type_obj) => {
                                // Create an exception instance from the type
                                use dx_py_core::pyexception::PyException;
                                PyValue::Exception(Arc::new(PyException::new(&type_obj.name, "")))
                            }
                            PyValue::Exception(_) => exc_value,
                            PyValue::Str(msg) => {
                                // String is treated as a RuntimeError message
                                use dx_py_core::pyexception::PyException;
                                PyValue::Exception(Arc::new(PyException::new(
                                    "RuntimeError",
                                    msg.to_string(),
                                )))
                            }
                            _ => {
                                return Err(InterpreterError::TypeError(format!(
                                    "exceptions must derive from BaseException, not '{}'",
                                    exc_value.type_name()
                                )));
                            }
                        }
                    }
                    2 => {
                        // raise exc from cause
                        let cause = frame.pop();
                        let exc_value = frame.pop();

                        // Convert exc_value to exception if needed
                        let mut exc = match exc_value {
                            PyValue::Type(type_obj) => {
                                use dx_py_core::pyexception::PyException;
                                PyException::new(&type_obj.name, "")
                            }
                            PyValue::Exception(e) => (*e).clone(),
                            _ => {
                                return Err(InterpreterError::TypeError(format!(
                                    "exceptions must derive from BaseException, not '{}'",
                                    exc_value.type_name()
                                )));
                            }
                        };

                        // Handle exception chaining based on cause value
                        match cause {
                            PyValue::Exception(cause_exc) => {
                                // raise X from Y - set __cause__ to Y
                                exc = exc.with_cause(cause_exc);
                            }
                            PyValue::None => {
                                // raise X from None - explicitly suppress context
                                // This sets __cause__ to None and __suppress_context__ to True
                                exc.set_suppress_context(true);
                            }
                            PyValue::Type(type_obj) => {
                                // raise X from ExceptionType - instantiate the cause
                                use dx_py_core::pyexception::PyException;
                                let cause_exc = Arc::new(PyException::new(&type_obj.name, ""));
                                exc = exc.with_cause(cause_exc);
                            }
                            _ => {
                                return Err(InterpreterError::TypeError(format!(
                                    "exception causes must be derived from BaseException or None, not '{}'",
                                    cause.type_name()
                                )));
                            }
                        }

                        PyValue::Exception(Arc::new(exc))
                    }
                    _ => {
                        return Err(InterpreterError::Runtime(format!(
                            "invalid RAISE_VARARGS argument: {}",
                            argc
                        )));
                    }
                };
                return Ok(DispatchResult::Exception(exc));
            }
            Opcode::Reraise => {
                let exc = frame.pop();
                return Ok(DispatchResult::Exception(exc));
            }
            Opcode::PushExcInfo => {
                let exc = frame.peek().clone();
                frame.push(exc);
            }
            Opcode::CheckExcMatch => {
                // CHECK_EXC_MATCH: Check if exception matches the given type
                // Stack: [exc, exc_type] -> [exc, bool]
                // The exception type can be:
                // - A string (exception type name)
                // - A Type object (exception class)
                // - A tuple of types (for multiple exception types)
                let exc_type = frame.pop();
                let exc = frame.peek();
                let matches = match exc {
                    PyValue::Exception(e) => {
                        match &exc_type {
                            // Match against string type name
                            PyValue::Str(type_name) => e.is_instance(type_name),
                            // Match against Type object
                            PyValue::Type(type_obj) => e.is_instance(&type_obj.name),
                            // Match against tuple of types (for `except (TypeError, ValueError):`)
                            PyValue::Tuple(types) => types.to_vec().iter().any(|t| match t {
                                PyValue::Str(name) => e.is_instance(name),
                                PyValue::Type(type_obj) => e.is_instance(&type_obj.name),
                                _ => false,
                            }),
                            _ => false,
                        }
                    }
                    _ => false,
                };
                frame.push(PyValue::Bool(matches));
            }

            // Context managers
            Opcode::SetupWith => {
                let handler = arg.ok_or_else(|| {
                    InterpreterError::Runtime("SETUP_WITH requires argument".into())
                })?;
                frame.push_block(dx_py_core::pyframe::BlockType::With, handler);
            }
            Opcode::BeforeWith => {
                // BEFORE_WITH: Prepare context manager for 'with' statement
                // Stack: [context_manager] -> [__exit__, result_of_enter]
                //
                // The context manager is on top of the stack.
                // We need to:
                // 1. Get __exit__ method and push it (for later cleanup)
                // 2. Call __enter__ and push its result (for 'as' binding)
                //
                // This matches CPython's behavior where __exit__ is saved on the stack
                // for later use when exiting the with block.

                let context_manager = frame.pop();

                // Get __exit__ method first (we need to save it for cleanup)
                let exit_method = match &context_manager {
                    PyValue::Instance(instance) => {
                        instance.get_attr("__exit__").ok_or_else(|| {
                            InterpreterError::AttributeError(format!(
                                "'{}' object does not support the context manager protocol (missing __exit__)",
                                instance.class_name()
                            ))
                        })?
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object does not support the context manager protocol",
                            context_manager.type_name()
                        )));
                    }
                };

                // Get __enter__ method
                let enter_method = match &context_manager {
                    PyValue::Instance(instance) => {
                        instance.get_attr("__enter__").ok_or_else(|| {
                            InterpreterError::AttributeError(format!(
                                "'{}' object does not support the context manager protocol (missing __enter__)",
                                instance.class_name()
                            ))
                        })?
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object does not support the context manager protocol",
                            context_manager.type_name()
                        )));
                    }
                };

                // Call __enter__() with no arguments (self is already bound)
                let enter_result = match enter_method {
                    PyValue::BoundMethod(ref method) => self.call_bound_method(method, &[])?,
                    PyValue::Function(ref func) => {
                        // If it's an unbound function, we need to pass the instance
                        self.call_user_function(func, std::slice::from_ref(&context_manager))?
                    }
                    PyValue::Builtin(ref builtin) => builtin
                        .call(std::slice::from_ref(&context_manager))
                        .map_err(InterpreterError::RuntimeError)?,
                    _ => {
                        return Err(InterpreterError::TypeError(
                            "__enter__ is not callable".into(),
                        ));
                    }
                };

                // Push __exit__ method (for cleanup - will be called later)
                frame.push(exit_method);
                // Push result of __enter__ (for 'as' binding)
                frame.push(enter_result);
            }
            Opcode::WithExceptStart => {
                // WITH_EXCEPT_START: Call __exit__ with exception info
                // Stack: [..., __exit__, exc_type, exc_value, exc_tb] -> [..., __exit__, exc_type, exc_value, exc_tb, result]
                //
                // This is called when an exception occurs in the with block.
                // We need to call __exit__(exc_type, exc_value, exc_tb) and check if it returns True
                // to suppress the exception.
                //
                // Note: We don't pop __exit__ or the exception info - they stay on the stack
                // for potential re-raise or cleanup.

                // Get exception info from stack (don't pop)
                let exc_tb = frame.peek_n(0).clone();
                let exc_value = frame.peek_n(1).clone();
                let exc_type = frame.peek_n(2).clone();
                let exit_method = frame.peek_n(3).clone();

                // Call __exit__(exc_type, exc_value, exc_tb)
                let suppress_result = match exit_method {
                    PyValue::BoundMethod(ref method) => {
                        self.call_bound_method(method, &[exc_type, exc_value, exc_tb])?
                    }
                    PyValue::Function(ref func) => {
                        self.call_user_function(func, &[exc_type, exc_value, exc_tb])?
                    }
                    PyValue::Builtin(ref builtin) => builtin
                        .call(&[exc_type, exc_value, exc_tb])
                        .map_err(InterpreterError::RuntimeError)?,
                    _ => {
                        return Err(InterpreterError::TypeError("__exit__ is not callable".into()));
                    }
                };

                // Push the result of __exit__ (True means suppress exception)
                frame.push(suppress_result);
            }

            // Async context managers
            Opcode::SetupAsyncWith => {
                let handler = arg.ok_or_else(|| {
                    InterpreterError::Runtime("SETUP_ASYNC_WITH requires argument".into())
                })?;
                frame.push_block(dx_py_core::pyframe::BlockType::With, handler);
            }
            Opcode::BeforeAsyncWith => {
                // BEFORE_ASYNC_WITH: Prepare async context manager for 'async with' statement
                // Stack: [context_manager] -> [__aexit__, result_of_aenter]
                //
                // Similar to BEFORE_WITH but for async context managers
                // We need to:
                // 1. Get __aexit__ method and push it (for later cleanup)
                // 2. Call __aenter__ and await its result (for 'as' binding)

                let context_manager = frame.pop();

                // Get __aexit__ method first (we need to save it for cleanup)
                let aexit_method = match &context_manager {
                    PyValue::Instance(instance) => {
                        instance.get_attr("__aexit__").ok_or_else(|| {
                            InterpreterError::AttributeError(format!(
                                "'{}' object does not support the async context manager protocol (missing __aexit__)",
                                instance.class_name()
                            ))
                        })?
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object does not support the async context manager protocol",
                            context_manager.type_name()
                        )));
                    }
                };

                // Get __aenter__ method
                let aenter_method = match &context_manager {
                    PyValue::Instance(instance) => {
                        instance.get_attr("__aenter__").ok_or_else(|| {
                            InterpreterError::AttributeError(format!(
                                "'{}' object does not support the async context manager protocol (missing __aenter__)",
                                instance.class_name()
                            ))
                        })?
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(format!(
                            "'{}' object does not support the async context manager protocol",
                            context_manager.type_name()
                        )));
                    }
                };

                // Call __aenter__() - this returns a coroutine that we need to await
                let aenter_result = match aenter_method {
                    PyValue::BoundMethod(ref method) => self.call_bound_method(method, &[])?,
                    PyValue::Function(ref func) => {
                        self.call_user_function(func, std::slice::from_ref(&context_manager))?
                    }
                    PyValue::Builtin(ref builtin) => builtin
                        .call(std::slice::from_ref(&context_manager))
                        .map_err(InterpreterError::RuntimeError)?,
                    _ => {
                        return Err(InterpreterError::TypeError(
                            "__aenter__ is not callable".into(),
                        ));
                    }
                };

                // Push __aexit__ method (for cleanup - will be called later)
                frame.push(aexit_method);
                // Push result of __aenter__ (should be awaited by the compiler-generated code)
                frame.push(aenter_result);
            }

            // Import operations
            Opcode::ImportName => {
                let name_idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("IMPORT_NAME requires argument".into())
                })?;

                let module_name = self.names.get(name_idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", name_idx))
                })?;

                // Pop fromlist and level from stack (pushed by compiler)
                let _fromlist = frame.pop();
                let _level = frame.pop();

                // Import the module using the module system
                let module = self.import_module(module_name)?;
                frame.push(PyValue::Module(module));
            }
            Opcode::ImportFrom => {
                let name_idx = arg.ok_or_else(|| {
                    InterpreterError::Runtime("IMPORT_FROM requires argument".into())
                })?;

                let attr_name = self.names.get(name_idx).ok_or_else(|| {
                    InterpreterError::NameError(format!("name index {} out of range", name_idx))
                })?;

                // Module is on top of stack (don't pop it - IMPORT_FROM leaves module on stack)
                let module = frame.peek().clone();

                if let PyValue::Module(m) = module {
                    // Get attribute from module's dict
                    let value =
                        m.dict.get(attr_name).map(|v| v.value().clone()).ok_or_else(|| {
                            InterpreterError::ImportError(format!(
                                "cannot import name '{}' from '{}'",
                                attr_name, m.name
                            ))
                        })?;
                    frame.push(value);
                } else {
                    return Err(InterpreterError::TypeError(format!(
                        "expected module, got {}",
                        module.type_name()
                    )));
                }
            }
            Opcode::ImportStar => {
                // Import all public names from module
                let module = frame.pop();

                if let PyValue::Module(m) = module {
                    // Get __all__ if defined, otherwise get all public names
                    let names_to_import: Vec<String> = if let Some(all) = m.dict.get("__all__") {
                        match all.value() {
                            PyValue::List(list) => list
                                .to_vec()
                                .iter()
                                .filter_map(|v| {
                                    if let PyValue::Str(s) = v {
                                        Some(s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                            PyValue::Tuple(tuple) => tuple
                                .to_vec()
                                .iter()
                                .filter_map(|v| {
                                    if let PyValue::Str(s) = v {
                                        Some(s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                            _ => {
                                // __all__ is not a list/tuple, import all public names
                                m.dict
                                    .iter()
                                    .map(|r| r.key().clone())
                                    .filter(|name| !name.starts_with('_'))
                                    .collect()
                            }
                        }
                    } else {
                        // No __all__, import all public names (not starting with _)
                        m.dict
                            .iter()
                            .map(|r| r.key().clone())
                            .filter(|name| !name.starts_with('_'))
                            .collect()
                    };

                    // Import each name into globals
                    for name in names_to_import {
                        if let Some(value) = m.dict.get(&name) {
                            self.globals.setitem(
                                PyKey::Str(Arc::from(name.as_str())),
                                value.value().clone(),
                            );
                        }
                    }
                } else {
                    return Err(InterpreterError::TypeError(format!(
                        "expected module, got {}",
                        module.type_name()
                    )));
                }
            }

            // Special opcodes
            Opcode::Resume => {
                // Resume is used for generators/coroutines - just continue
            }
            Opcode::Cache => {
                // Cache is used for inline caching - skip the cache slot
                let _cache_size = arg.unwrap_or(0);
            }
            Opcode::Precall => {
                // Precall is used for call preparation - just continue
            }
            Opcode::BinaryOp => {
                // Generic binary operation - arg specifies which operation
                let op = arg.ok_or_else(|| {
                    InterpreterError::Runtime("BINARY_OP requires argument".into())
                })?;
                let b = frame.pop();
                let a = frame.pop();
                let result = match op {
                    0 => self.binary_add(&a, &b)?,
                    1 => self.binary_and(&a, &b)?,
                    2 => self.binary_floordiv(&a, &b)?,
                    3 => self.binary_lshift(&a, &b)?,
                    5 => self.binary_mul(&a, &b)?,
                    6 => self.binary_mod(&a, &b)?,
                    7 => self.binary_or(&a, &b)?,
                    8 => self.binary_pow(&a, &b)?,
                    9 => self.binary_rshift(&a, &b)?,
                    10 => self.binary_sub(&a, &b)?,
                    11 => self.binary_div(&a, &b)?,
                    12 => self.binary_xor(&a, &b)?,
                    _ => {
                        return Err(InterpreterError::Runtime(format!("Unknown binary op: {}", op)))
                    }
                };
                frame.push(result);
            }
            Opcode::CompareOp => {
                // Generic compare operation - arg specifies which comparison
                let op = arg.ok_or_else(|| {
                    InterpreterError::Runtime("COMPARE_OP requires argument".into())
                })?;
                let b = frame.pop();
                let a = frame.pop();
                let result = match op {
                    0 => PyValue::Bool(self.compare_lt(&a, &b)?),
                    1 => PyValue::Bool(self.compare_lt(&a, &b)? || self.compare_eq(&a, &b)),
                    2 => PyValue::Bool(self.compare_eq(&a, &b)),
                    3 => PyValue::Bool(!self.compare_eq(&a, &b)),
                    4 => PyValue::Bool(self.compare_lt(&b, &a)?),
                    5 => PyValue::Bool(self.compare_lt(&b, &a)? || self.compare_eq(&a, &b)),
                    6 => PyValue::Bool(self.compare_in(&a, &b)?),
                    7 => PyValue::Bool(!self.compare_in(&a, &b)?),
                    8 => PyValue::Bool(self.compare_is(&a, &b)),
                    9 => PyValue::Bool(!self.compare_is(&a, &b)),
                    _ => {
                        return Err(InterpreterError::Runtime(format!(
                            "Unknown compare op: {}",
                            op
                        )))
                    }
                };
                frame.push(result);
            }
            Opcode::Extended => {
                // Extended opcode - read 4-byte argument and apply to next opcode
                // The extended arg is used to extend the argument of the following opcode
                // For now, we just skip it and let the next opcode handle it
                // This is a simplification - proper implementation would accumulate the extended arg
            }
        }

        Ok(DispatchResult::Continue)
    }

    // Binary operation helpers
    fn binary_add(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x + y)),
            (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x + y)),
            (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(*x as f64 + y)),
            (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x + *y as f64)),
            (PyValue::Str(x), PyValue::Str(y)) => {
                let mut s = x.to_string();
                s.push_str(y);
                Ok(PyValue::Str(Arc::from(s)))
            }
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for +: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_sub(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x - y)),
            (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x - y)),
            (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(*x as f64 - y)),
            (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x - *y as f64)),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for -: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_mul(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x * y)),
            (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x * y)),
            (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(*x as f64 * y)),
            (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x * *y as f64)),
            (PyValue::Str(s), PyValue::Int(n)) | (PyValue::Int(n), PyValue::Str(s)) => {
                if *n <= 0 {
                    Ok(PyValue::Str(Arc::from("")))
                } else {
                    Ok(PyValue::Str(Arc::from(s.repeat(*n as usize))))
                }
            }
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for *: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_div(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => {
                if *y == 0 {
                    Err(InterpreterError::ValueError("division by zero".into()))
                } else {
                    Ok(PyValue::Float(*x as f64 / *y as f64))
                }
            }
            (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x / y)),
            (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(*x as f64 / y)),
            (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x / *y as f64)),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for /: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_floordiv(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => {
                if *y == 0 {
                    Err(InterpreterError::ValueError("integer division by zero".into()))
                } else {
                    // Python floor division: rounds toward negative infinity
                    // This differs from Rust's div_euclid which uses Euclidean division
                    Ok(PyValue::Int((*x as f64 / *y as f64).floor() as i64))
                }
            }
            (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float((x / y).floor())),
            (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float((*x as f64 / y).floor())),
            (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float((x / *y as f64).floor())),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for //: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_mod(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => {
                if *y == 0 {
                    Err(InterpreterError::ValueError("integer modulo by zero".into()))
                } else {
                    // Python modulo: result has same sign as divisor
                    // Formula: ((a % b) + b) % b
                    let result = ((*x % *y) + *y) % *y;
                    Ok(PyValue::Int(result))
                }
            }
            (PyValue::Float(x), PyValue::Float(y)) => {
                // Python float modulo: result has same sign as divisor
                let result = ((x % y) + y) % y;
                Ok(PyValue::Float(result))
            }
            (PyValue::Int(x), PyValue::Float(y)) => {
                let xf = *x as f64;
                let result = ((xf % y) + y) % y;
                Ok(PyValue::Float(result))
            }
            (PyValue::Float(x), PyValue::Int(y)) => {
                let yf = *y as f64;
                let result = ((x % yf) + yf) % yf;
                Ok(PyValue::Float(result))
            }
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for %: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_pow(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => {
                if *y >= 0 {
                    Ok(PyValue::Int(x.pow(*y as u32)))
                } else {
                    Ok(PyValue::Float((*x as f64).powi(*y as i32)))
                }
            }
            (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x.powf(*y))),
            (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float((*x as f64).powf(*y))),
            (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x.powi(*y as i32))),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for **: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_and(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x & y)),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for &: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_or(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x | y)),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for |: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_xor(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x ^ y)),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for ^: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_lshift(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x << y)),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for <<: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn binary_rshift(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<PyValue> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x >> y)),
            _ => Err(InterpreterError::TypeError(format!(
                "unsupported operand type(s) for >>: '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    // Unary operation helpers
    fn unary_neg(&self, a: &PyValue) -> InterpreterResult<PyValue> {
        match a {
            PyValue::Int(x) => Ok(PyValue::Int(-x)),
            PyValue::Float(x) => Ok(PyValue::Float(-x)),
            _ => Err(InterpreterError::TypeError(format!(
                "bad operand type for unary -: '{}'",
                a.type_name()
            ))),
        }
    }

    fn unary_pos(&self, a: &PyValue) -> InterpreterResult<PyValue> {
        match a {
            PyValue::Int(x) => Ok(PyValue::Int(*x)),
            PyValue::Float(x) => Ok(PyValue::Float(*x)),
            _ => Err(InterpreterError::TypeError(format!(
                "bad operand type for unary +: '{}'",
                a.type_name()
            ))),
        }
    }

    fn unary_invert(&self, a: &PyValue) -> InterpreterResult<PyValue> {
        match a {
            PyValue::Int(x) => Ok(PyValue::Int(!x)),
            _ => Err(InterpreterError::TypeError(format!(
                "bad operand type for unary ~: '{}'",
                a.type_name()
            ))),
        }
    }

    // Comparison helpers
    fn compare_eq(&self, a: &PyValue, b: &PyValue) -> bool {
        match (a, b) {
            (PyValue::None, PyValue::None) => true,
            (PyValue::Bool(x), PyValue::Bool(y)) => x == y,
            (PyValue::Int(x), PyValue::Int(y)) => x == y,
            (PyValue::Float(x), PyValue::Float(y)) => x == y,
            (PyValue::Int(x), PyValue::Float(y)) => (*x as f64) == *y,
            (PyValue::Float(x), PyValue::Int(y)) => *x == (*y as f64),
            (PyValue::Str(x), PyValue::Str(y)) => x == y,
            _ => false,
        }
    }

    fn compare_lt(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<bool> {
        match (a, b) {
            (PyValue::Int(x), PyValue::Int(y)) => Ok(x < y),
            (PyValue::Float(x), PyValue::Float(y)) => Ok(x < y),
            (PyValue::Int(x), PyValue::Float(y)) => Ok((*x as f64) < *y),
            (PyValue::Float(x), PyValue::Int(y)) => Ok(*x < (*y as f64)),
            (PyValue::Str(x), PyValue::Str(y)) => Ok(x < y),
            _ => Err(InterpreterError::TypeError(format!(
                "'<' not supported between instances of '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn compare_is(&self, a: &PyValue, b: &PyValue) -> bool {
        match (a, b) {
            (PyValue::None, PyValue::None) => true,
            _ => std::ptr::eq(a, b),
        }
    }

    fn compare_in(&self, a: &PyValue, b: &PyValue) -> InterpreterResult<bool> {
        match b {
            PyValue::List(list) => Ok(list.to_vec().iter().any(|x| self.compare_eq(a, x))),
            PyValue::Set(set) => Ok(set.contains(a)),
            PyValue::Tuple(tuple) => Ok(tuple.to_vec().iter().any(|x| self.compare_eq(a, x))),
            PyValue::Str(s) => {
                if let PyValue::Str(needle) = a {
                    Ok(s.contains(&**needle))
                } else {
                    Err(InterpreterError::TypeError("'in' requires string as left operand".into()))
                }
            }
            PyValue::Dict(d) => {
                let key = match a {
                    PyValue::Str(s) => PyKey::Str(s.clone()),
                    PyValue::Int(i) => PyKey::Int(*i),
                    _ => return Err(InterpreterError::TypeError("unhashable type".into())),
                };
                Ok(d.getitem(&key).is_ok())
            }
            _ => Err(InterpreterError::TypeError(format!(
                "argument of type '{}' is not iterable",
                b.type_name()
            ))),
        }
    }

    // Subscript helpers
    fn subscript(&self, container: &PyValue, index: &PyValue) -> InterpreterResult<PyValue> {
        match (container, index) {
            (PyValue::List(list), PyValue::Int(i)) => {
                let items = list.to_vec();
                let idx = if *i < 0 { items.len() as i64 + i } else { *i } as usize;
                items
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| InterpreterError::IndexError("list index out of range".into()))
            }
            (PyValue::Tuple(tuple), PyValue::Int(i)) => {
                let items = tuple.to_vec();
                let idx = if *i < 0 { items.len() as i64 + i } else { *i } as usize;
                items
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| InterpreterError::IndexError("tuple index out of range".into()))
            }
            (PyValue::Str(s), PyValue::Int(i)) => {
                let chars: Vec<char> = s.chars().collect();
                let idx = if *i < 0 { chars.len() as i64 + i } else { *i } as usize;
                chars
                    .get(idx)
                    .map(|c| PyValue::Str(Arc::from(c.to_string())))
                    .ok_or_else(|| InterpreterError::IndexError("string index out of range".into()))
            }
            (PyValue::Dict(d), key) => {
                let py_key = match key {
                    PyValue::Str(s) => PyKey::Str(s.clone()),
                    PyValue::Int(i) => PyKey::Int(*i),
                    _ => return Err(InterpreterError::TypeError("unhashable type".into())),
                };
                d.getitem(&py_key)
                    .map_err(|_| InterpreterError::KeyError("key not found".into()))
            }
            _ => Err(InterpreterError::TypeError(format!(
                "'{}' object is not subscriptable",
                container.type_name()
            ))),
        }
    }

    fn store_subscript(
        &self,
        container: &PyValue,
        index: &PyValue,
        value: PyValue,
    ) -> InterpreterResult<()> {
        match (container, index) {
            (PyValue::List(list), PyValue::Int(i)) => {
                let idx = if *i < 0 { list.len() as i64 + i } else { *i };
                list.setitem(idx, value)
                    .map_err(|e| InterpreterError::IndexError(e.to_string()))?;
                Ok(())
            }
            (PyValue::Dict(d), key) => {
                let py_key = match key {
                    PyValue::Str(s) => PyKey::Str(s.clone()),
                    PyValue::Int(i) => PyKey::Int(*i),
                    _ => return Err(InterpreterError::TypeError("unhashable type".into())),
                };
                d.setitem(py_key, value);
                Ok(())
            }
            _ => Err(InterpreterError::TypeError(format!(
                "'{}' object does not support item assignment",
                container.type_name()
            ))),
        }
    }

    /// Call a user-defined function with the given arguments
    fn call_user_function(
        &self,
        func: &Arc<PyFunction>,
        args: &[PyValue],
    ) -> InterpreterResult<PyValue> {
        // Record the function call for hot function detection
        // When the call count reaches JIT_COMPILATION_THRESHOLD (100),
        // the function becomes a candidate for JIT compilation
        let call_count = func.record_call();
        
        // Check if this function just became hot (exactly at threshold)
        // This is where JIT compilation would be triggered
        if func.just_became_hot() {
            // Log that this function is now hot and ready for JIT compilation
            // In a full implementation, this would trigger actual JIT compilation
            // via the TieredJit::on_function_call or similar mechanism
            #[cfg(debug_assertions)]
            eprintln!(
                "[JIT] Function '{}' reached {} calls - eligible for JIT compilation",
                func.qualname,
                call_count
            );
        }
        
        // Bind arguments to locals
        let locals = self.bind_arguments(func, args, None)?;

        // Get the code object from the function's closure or from the function itself
        // The function should have been created with a PyCode that contains the bytecode
        // We need to find the bytecode for this function

        // Check if the function has a closure with a code object
        // For now, we'll look for the code in the constants pool or use the function's stored code
        let (bytecode, constants, names) = self.get_function_code(func)?;

        // Create a new frame for the function
        let mut new_frame = PyFrame::new(Arc::clone(func), None);

        // Set locals from bound arguments
        for (i, value) in locals.into_iter().enumerate() {
            new_frame.set_local(i, value);
        }

        // Check if this is a generator function
        if func.flags.is_generator {
            // Return a generator object instead of executing the function
            let generator = PyGenerator::new(Arc::clone(func), new_frame);
            return Ok(PyValue::Generator(Arc::new(generator)));
        }

        // Check if this is a coroutine function (async def)
        if func.flags.is_coroutine {
            // Return a coroutine object instead of executing the function
            let coroutine = PyCoroutine::new(Arc::clone(func), new_frame);
            return Ok(PyValue::Coroutine(Arc::new(coroutine)));
        }

        // Create a new dispatcher for the function's bytecode and execute
        let func_dispatcher = Dispatcher::with_globals(
            bytecode,
            constants,
            names,
            Arc::clone(&self.globals),
            self.builtins.clone(),
        );

        func_dispatcher.execute(&mut new_frame)
    }

    /// Call a bound method with the given arguments
    fn call_bound_method(
        &self,
        method: &dx_py_core::types::BoundMethod,
        args: &[PyValue],
    ) -> InterpreterResult<PyValue> {
        use dx_py_core::types::BoundMethod;

        match method {
            BoundMethod::Instance {
                method: inner_method,
                instance,
            } => {
                // Prepend instance (self) to args
                let mut full_args = vec![PyValue::Instance(Arc::clone(instance))];
                full_args.extend(args.iter().cloned());

                // Call the underlying method
                match inner_method.as_ref() {
                    PyValue::Function(func) => self.call_user_function(func, &full_args),
                    PyValue::Builtin(builtin) => {
                        builtin.call(&full_args).map_err(InterpreterError::RuntimeError)
                    }
                    _ => Err(InterpreterError::TypeError(
                        "bound method's underlying object is not callable".into(),
                    )),
                }
            }
            BoundMethod::Class {
                method: inner_method,
                class,
            } => {
                // Prepend class to args
                let mut full_args = vec![PyValue::Type(Arc::clone(class))];
                full_args.extend(args.iter().cloned());

                match inner_method.as_ref() {
                    PyValue::Function(func) => self.call_user_function(func, &full_args),
                    PyValue::Builtin(builtin) => {
                        builtin.call(&full_args).map_err(InterpreterError::RuntimeError)
                    }
                    _ => Err(InterpreterError::TypeError(
                        "class method's underlying object is not callable".into(),
                    )),
                }
            }
            BoundMethod::Static {
                method: inner_method,
            }
            | BoundMethod::Unbound {
                method: inner_method,
            } => {
                // No binding, just call with original args
                match inner_method.as_ref() {
                    PyValue::Function(func) => self.call_user_function(func, args),
                    PyValue::Builtin(builtin) => {
                        builtin.call(args).map_err(InterpreterError::RuntimeError)
                    }
                    _ => Err(InterpreterError::TypeError(
                        "static/unbound method's underlying object is not callable".into(),
                    )),
                }
            }
            BoundMethod::String { value, method: method_name } => {
                // Call string method
                self.call_string_method(value, method_name, args)
            }
            BoundMethod::List { value, method: method_name } => {
                // Call list method
                self.call_list_method(value, method_name, args)
            }
            BoundMethod::Dict { value, method: method_name } => {
                // Call dict method
                self.call_dict_method(value, method_name, args)
            }
        }
    }

    /// Call a string method
    fn call_string_method(
        &self,
        value: &Arc<str>,
        method_name: &str,
        args: &[PyValue],
    ) -> InterpreterResult<PyValue> {
        use dx_py_core::PyStr;
        
        let s = PyStr::from_arc(Arc::clone(value));
        
        match method_name {
            "upper" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "upper() takes no arguments".into(),
                    ));
                }
                Ok(PyValue::Str(Arc::from(s.upper().as_str())))
            }
            "lower" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "lower() takes no arguments".into(),
                    ));
                }
                Ok(PyValue::Str(Arc::from(s.lower().as_str())))
            }
            "strip" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "strip() takes no arguments".into(),
                    ));
                }
                Ok(PyValue::Str(Arc::from(s.strip().as_str())))
            }
            "lstrip" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "lstrip() takes no arguments".into(),
                    ));
                }
                Ok(PyValue::Str(Arc::from(s.lstrip().as_str())))
            }
            "rstrip" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "rstrip() takes no arguments".into(),
                    ));
                }
                Ok(PyValue::Str(Arc::from(s.rstrip().as_str())))
            }
            "split" => {
                match args.len() {
                    0 => {
                        // Split on whitespace
                        let parts = s.split(None);
                        let list_items: Vec<PyValue> = parts
                            .into_iter()
                            .map(|p| PyValue::Str(Arc::from(p.as_str())))
                            .collect();
                        Ok(PyValue::List(Arc::new(dx_py_core::PyList::from_values(list_items))))
                    }
                    1 => {
                        // Split on separator
                        if let PyValue::Str(sep) = &args[0] {
                            let sep_str = PyStr::from_arc(Arc::clone(sep));
                            let parts = s.split(Some(&sep_str));
                            let list_items: Vec<PyValue> = parts
                                .into_iter()
                                .map(|p| PyValue::Str(Arc::from(p.as_str())))
                                .collect();
                            Ok(PyValue::List(Arc::new(dx_py_core::PyList::from_values(list_items))))
                        } else {
                            Err(InterpreterError::TypeError(
                                "split() argument must be str".into(),
                            ))
                        }
                    }
                    _ => Err(InterpreterError::TypeError(
                        "split() takes at most 1 argument".into(),
                    )),
                }
            }
            "join" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "join() takes exactly 1 argument".into(),
                    ));
                }
                match &args[0] {
                    PyValue::List(list) => {
                        let items = list.to_vec();
                        let str_items: Result<Vec<PyStr>, _> = items
                            .iter()
                            .map(|item| {
                                if let PyValue::Str(s) = item {
                                    Ok(PyStr::from_arc(Arc::clone(s)))
                                } else {
                                    Err(InterpreterError::TypeError(
                                        "sequence item: expected str".into(),
                                    ))
                                }
                            })
                            .collect();
                        let str_items = str_items?;
                        let result = s.join(&str_items);
                        Ok(PyValue::Str(Arc::from(result.as_str())))
                    }
                    PyValue::Tuple(tuple) => {
                        let items = tuple.to_vec();
                        let str_items: Result<Vec<PyStr>, _> = items
                            .iter()
                            .map(|item| {
                                if let PyValue::Str(s) = item {
                                    Ok(PyStr::from_arc(Arc::clone(s)))
                                } else {
                                    Err(InterpreterError::TypeError(
                                        "sequence item: expected str".into(),
                                    ))
                                }
                            })
                            .collect();
                        let str_items = str_items?;
                        let result = s.join(&str_items);
                        Ok(PyValue::Str(Arc::from(result.as_str())))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "join() argument must be an iterable".into(),
                    )),
                }
            }
            "replace" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(InterpreterError::TypeError(
                        "replace() takes 2 or 3 arguments".into(),
                    ));
                }
                let (old, new) = match (&args[0], &args[1]) {
                    (PyValue::Str(old), PyValue::Str(new)) => {
                        (PyStr::from_arc(Arc::clone(old)), PyStr::from_arc(Arc::clone(new)))
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(
                            "replace() arguments must be str".into(),
                        ));
                    }
                };
                let count = if args.len() == 3 {
                    if let PyValue::Int(n) = &args[2] {
                        Some(*n as usize)
                    } else {
                        return Err(InterpreterError::TypeError(
                            "replace() count must be int".into(),
                        ));
                    }
                } else {
                    None
                };
                let result = s.replace_count(&old, &new, count);
                Ok(PyValue::Str(Arc::from(result.as_str())))
            }
            "startswith" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "startswith() takes exactly 1 argument".into(),
                    ));
                }
                if let PyValue::Str(prefix) = &args[0] {
                    let prefix_str = PyStr::from_arc(Arc::clone(prefix));
                    Ok(PyValue::Bool(s.startswith(&prefix_str)))
                } else {
                    Err(InterpreterError::TypeError(
                        "startswith() argument must be str".into(),
                    ))
                }
            }
            "endswith" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "endswith() takes exactly 1 argument".into(),
                    ));
                }
                if let PyValue::Str(suffix) = &args[0] {
                    let suffix_str = PyStr::from_arc(Arc::clone(suffix));
                    Ok(PyValue::Bool(s.endswith(&suffix_str)))
                } else {
                    Err(InterpreterError::TypeError(
                        "endswith() argument must be str".into(),
                    ))
                }
            }
            "find" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "find() takes exactly 1 argument".into(),
                    ));
                }
                if let PyValue::Str(sub) = &args[0] {
                    let sub_str = PyStr::from_arc(Arc::clone(sub));
                    Ok(PyValue::Int(s.find_index(&sub_str)))
                } else {
                    Err(InterpreterError::TypeError(
                        "find() argument must be str".into(),
                    ))
                }
            }
            "format" => {
                // Basic format implementation - just return the string for now
                // Full format implementation would require parsing format specifiers
                Ok(PyValue::Str(Arc::clone(value)))
            }
            _ => Err(InterpreterError::AttributeError(format!(
                "'str' object has no attribute '{}'",
                method_name
            ))),
        }
    }

    /// Call a list method
    fn call_list_method(
        &self,
        value: &Arc<dx_py_core::PyList>,
        method_name: &str,
        args: &[PyValue],
    ) -> InterpreterResult<PyValue> {
        match method_name {
            "append" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "append() takes exactly 1 argument".into(),
                    ));
                }
                value.append(args[0].clone());
                Ok(PyValue::None)
            }
            "extend" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "extend() takes exactly 1 argument".into(),
                    ));
                }
                match &args[0] {
                    PyValue::List(other) => {
                        value.extend(other.to_vec());
                        Ok(PyValue::None)
                    }
                    PyValue::Tuple(tuple) => {
                        value.extend(tuple.to_vec());
                        Ok(PyValue::None)
                    }
                    _ => Err(InterpreterError::TypeError(
                        "extend() argument must be iterable".into(),
                    )),
                }
            }
            "insert" => {
                if args.len() != 2 {
                    return Err(InterpreterError::TypeError(
                        "insert() takes exactly 2 arguments".into(),
                    ));
                }
                if let PyValue::Int(index) = &args[0] {
                    value.insert(*index, args[1].clone());
                    Ok(PyValue::None)
                } else {
                    Err(InterpreterError::TypeError(
                        "insert() first argument must be int".into(),
                    ))
                }
            }
            "remove" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "remove() takes exactly 1 argument".into(),
                    ));
                }
                value.remove(&args[0]).map_err(|e| InterpreterError::ValueError(e.to_string()))?;
                Ok(PyValue::None)
            }
            "pop" => {
                let index = match args.len() {
                    0 => None,
                    1 => {
                        if let PyValue::Int(i) = &args[0] {
                            Some(*i)
                        } else {
                            return Err(InterpreterError::TypeError(
                                "pop() argument must be int".into(),
                            ));
                        }
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(
                            "pop() takes at most 1 argument".into(),
                        ));
                    }
                };
                value.pop(index).map_err(|e| InterpreterError::IndexError(e.to_string()))
            }
            "clear" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "clear() takes no arguments".into(),
                    ));
                }
                value.clear();
                Ok(PyValue::None)
            }
            "index" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "index() takes exactly 1 argument".into(),
                    ));
                }
                let idx = value.index(&args[0]).map_err(|e| InterpreterError::ValueError(e.to_string()))?;
                Ok(PyValue::Int(idx as i64))
            }
            "count" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "count() takes exactly 1 argument".into(),
                    ));
                }
                let cnt = value.count(&args[0]);
                Ok(PyValue::Int(cnt as i64))
            }
            "sort" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "sort() takes no arguments".into(),
                    ));
                }
                value.sort().map_err(|e| InterpreterError::TypeError(e.to_string()))?;
                Ok(PyValue::None)
            }
            "reverse" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "reverse() takes no arguments".into(),
                    ));
                }
                value.reverse();
                Ok(PyValue::None)
            }
            "copy" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "copy() takes no arguments".into(),
                    ));
                }
                Ok(PyValue::List(Arc::new(dx_py_core::PyList::from_values(value.to_vec()))))
            }
            _ => Err(InterpreterError::AttributeError(format!(
                "'list' object has no attribute '{}'",
                method_name
            ))),
        }
    }

    /// Call a dict method
    fn call_dict_method(
        &self,
        value: &Arc<dx_py_core::PyDict>,
        method_name: &str,
        args: &[PyValue],
    ) -> InterpreterResult<PyValue> {
        match method_name {
            "keys" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "keys() takes no arguments".into(),
                    ));
                }
                Ok(value.keys_list())
            }
            "values" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "values() takes no arguments".into(),
                    ));
                }
                Ok(value.values_list())
            }
            "items" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "items() takes no arguments".into(),
                    ));
                }
                Ok(value.items_list())
            }
            "get" => {
                match args.len() {
                    1 => {
                        // get(key) - default is None
                        let key = PyKey::from_value(&args[0])
                            .map_err(|e| InterpreterError::RuntimeError(e))?;
                        Ok(value.get_with_default(&key, PyValue::None))
                    }
                    2 => {
                        // get(key, default)
                        let key = PyKey::from_value(&args[0])
                            .map_err(|e| InterpreterError::RuntimeError(e))?;
                        Ok(value.get_with_default(&key, args[1].clone()))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "get() takes 1 or 2 arguments".into(),
                    )),
                }
            }
            "pop" => {
                match args.len() {
                    1 => {
                        // pop(key) - raises KeyError if not found
                        let key = PyKey::from_value(&args[0])
                            .map_err(|e| InterpreterError::RuntimeError(e))?;
                        value.pop_with_default(&key, None)
                            .map_err(|e| InterpreterError::RuntimeError(e))
                    }
                    2 => {
                        // pop(key, default)
                        let key = PyKey::from_value(&args[0])
                            .map_err(|e| InterpreterError::RuntimeError(e))?;
                        value.pop_with_default(&key, Some(args[1].clone()))
                            .map_err(|e| InterpreterError::RuntimeError(e))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "pop() takes 1 or 2 arguments".into(),
                    )),
                }
            }
            "update" => {
                if args.len() != 1 {
                    return Err(InterpreterError::TypeError(
                        "update() takes exactly 1 argument".into(),
                    ));
                }
                match &args[0] {
                    PyValue::Dict(other) => {
                        value.update_from(other);
                        Ok(PyValue::None)
                    }
                    _ => Err(InterpreterError::TypeError(
                        "update() argument must be a dict".into(),
                    )),
                }
            }
            "clear" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "clear() takes no arguments".into(),
                    ));
                }
                value.clear();
                Ok(PyValue::None)
            }
            "copy" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "copy() takes no arguments".into(),
                    ));
                }
                Ok(PyValue::Dict(Arc::new(value.copy())))
            }
            "setdefault" => {
                match args.len() {
                    1 => {
                        // setdefault(key) - default is None
                        let key = PyKey::from_value(&args[0])
                            .map_err(|e| InterpreterError::RuntimeError(e))?;
                        Ok(value.setdefault(key, PyValue::None))
                    }
                    2 => {
                        // setdefault(key, default)
                        let key = PyKey::from_value(&args[0])
                            .map_err(|e| InterpreterError::RuntimeError(e))?;
                        Ok(value.setdefault(key, args[1].clone()))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "setdefault() takes 1 or 2 arguments".into(),
                    )),
                }
            }
            "popitem" => {
                if !args.is_empty() {
                    return Err(InterpreterError::TypeError(
                        "popitem() takes no arguments".into(),
                    ));
                }
                let (key, val) = value.popitem()
                    .map_err(|e| InterpreterError::RuntimeError(e))?;
                Ok(PyValue::Tuple(Arc::new(dx_py_core::PyTuple::pair(key.to_value(), val))))
            }
            _ => Err(InterpreterError::AttributeError(format!(
                "dict method '{}' not implemented",
                method_name
            ))),
        }
    }

    /// Get the bytecode, constants, and names for a function
    fn get_function_code(
        &self,
        func: &Arc<PyFunction>,
    ) -> InterpreterResult<(Vec<u8>, Vec<PyValue>, Vec<String>)> {
        // The function's closure may contain a code object
        // Or we need to look it up from the constants pool

        // First, check if there's a code object in the closure
        for value in &func.closure {
            if let PyValue::Code(code) = value {
                return Ok((
                    code.code.to_vec(),
                    code.constants.to_vec(),
                    code.names.iter().map(|s| s.to_string()).collect(),
                ));
            }
        }

        // If not in closure, look for a code object in our constants pool
        // that matches the function's qualname
        for constant in &self.constants {
            if let PyValue::Code(code) = constant {
                if code.qualname.as_ref() == func.qualname || code.name.as_ref() == func.name {
                    return Ok((
                        code.code.to_vec(),
                        code.constants.to_vec(),
                        code.names.iter().map(|s| s.to_string()).collect(),
                    ));
                }
            }
        }

        Err(InterpreterError::Runtime(format!(
            "No bytecode found for function '{}'",
            func.qualname
        )))
    }

    /// Bind arguments to function parameters
    fn bind_arguments(
        &self,
        func: &Arc<PyFunction>,
        args: &[PyValue],
        kwargs: Option<&HashMap<String, PyValue>>,
    ) -> InterpreterResult<Vec<PyValue>> {
        let mut locals = vec![PyValue::None; func.code.num_locals as usize];
        let mut args_iter = args.iter();
        let mut varargs: Vec<PyValue> = Vec::new();
        let mut kwdict: HashMap<String, PyValue> = HashMap::new();
        let mut _positional_count = 0;

        // Process parameters
        for (i, param) in func.params.iter().enumerate() {
            match param.kind {
                ParameterKind::Positional | ParameterKind::PositionalOrKeyword => {
                    // Try to get from positional args first
                    if let Some(value) = args_iter.next() {
                        locals[i] = value.clone();
                        _positional_count += 1;
                    } else if let Some(kw) = kwargs {
                        // Try keyword argument
                        if let Some(value) = kw.get(&param.name) {
                            locals[i] = value.clone();
                        } else if let Some(default) = func.get_default(i) {
                            locals[i] = default.clone();
                        } else {
                            return Err(InterpreterError::TypeError(format!(
                                "{}() missing required argument: '{}'",
                                func.name, param.name
                            )));
                        }
                    } else if let Some(default) = func.get_default(i) {
                        locals[i] = default.clone();
                    } else {
                        return Err(InterpreterError::TypeError(format!(
                            "{}() missing required argument: '{}'",
                            func.name, param.name
                        )));
                    }
                }
                ParameterKind::VarPositional => {
                    // Collect remaining positional args into *args
                    varargs.extend(args_iter.by_ref().cloned());
                    locals[i] =
                        PyValue::Tuple(Arc::new(dx_py_core::PyTuple::from_values(varargs.clone())));
                }
                ParameterKind::KeywordOnly => {
                    // Must come from kwargs
                    if let Some(kw) = kwargs {
                        if let Some(value) = kw.get(&param.name) {
                            locals[i] = value.clone();
                        } else if let Some(default) = &param.default {
                            locals[i] = default.clone();
                        } else {
                            return Err(InterpreterError::TypeError(format!(
                                "{}() missing required keyword argument: '{}'",
                                func.name, param.name
                            )));
                        }
                    } else if let Some(default) = &param.default {
                        locals[i] = default.clone();
                    } else {
                        return Err(InterpreterError::TypeError(format!(
                            "{}() missing required keyword argument: '{}'",
                            func.name, param.name
                        )));
                    }
                }
                ParameterKind::VarKeyword => {
                    // Collect remaining kwargs into **kwargs
                    if let Some(kw) = kwargs {
                        for (k, v) in kw {
                            // Only include kwargs not already bound
                            let already_bound = func.params.iter().take(i).any(|p| &p.name == k);
                            if !already_bound {
                                kwdict.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    let dict = Arc::new(PyDict::new());
                    for (k, v) in kwdict.iter() {
                        dict.setitem(PyKey::Str(Arc::from(k.as_str())), v.clone());
                    }
                    locals[i] = PyValue::Dict(dict);
                }
            }
        }

        // Check for extra positional arguments
        if !func.flags.has_varargs {
            let remaining: Vec<_> = args_iter.collect();
            if !remaining.is_empty() {
                return Err(InterpreterError::TypeError(format!(
                    "{}() takes {} positional arguments but {} were given",
                    func.name,
                    func.max_positional_args().unwrap_or(0),
                    args.len()
                )));
            }
        }

        Ok(locals)
    }

    /// Instantiate a class - create a new instance and call __init__ if defined
    fn instantiate_class(
        &self,
        class: &Arc<dx_py_core::types::PyType>,
        args: &[PyValue],
    ) -> InterpreterResult<PyValue> {
        // Create a new instance of the class
        let instance = Arc::new(dx_py_core::types::PyInstance::new(Arc::clone(class)));

        // Check if the class has an __init__ method
        if let Some(init_method) = class.get_init() {
            // Prepare arguments: prepend the instance (self) to the args
            let mut init_args = vec![PyValue::Instance(Arc::clone(&instance))];
            init_args.extend(args.iter().cloned());

            // Call __init__ method
            match init_method {
                PyValue::Function(func) => {
                    // Call the __init__ function with self prepended
                    let _ = self.call_user_function(&func, &init_args)?;
                }
                PyValue::Builtin(builtin) => {
                    // Call builtin __init__
                    let _ = builtin.call(&init_args).map_err(InterpreterError::RuntimeError)?;
                }
                PyValue::BoundMethod(ref method) => {
                    // Call bound method __init__
                    let _ = self.call_bound_method(method, args)?;
                }
                _ => {
                    // __init__ is not callable - this is an error
                    return Err(InterpreterError::TypeError("__init__ is not callable".into()));
                }
            }
        }

        // Return the instance
        Ok(PyValue::Instance(instance))
    }

    /// Import a module by name
    ///
    /// This implements the module import system with caching.
    /// Modules are cached in self.modules (equivalent to sys.modules).
    fn import_module(&self, name: &str) -> InterpreterResult<Arc<PyModule>> {
        // Check if module is already cached
        if let Some(module) = self.modules.get(name) {
            return Ok(Arc::clone(&module));
        }

        // Handle os.path as a special submodule
        if name == "os.path" {
            let module = self.create_builtin_module("os.path")?;
            let module_arc = Arc::new(module);
            self.modules.insert(name.to_string(), Arc::clone(&module_arc));
            return Ok(module_arc);
        }

        // List of built-in modules that we support
        let builtin_modules = [
            "sys",
            "builtins",
            "os",
            "io",
            "json",
            "re",
            "math",
            "collections",
            "itertools",
            "functools",
            "typing",
            "pathlib",
            "datetime",
            "time",
            "random",
            "string",
        ];

        // Check if it's a built-in module
        if builtin_modules.contains(&name) {
            let module = self.create_builtin_module(name)?;
            let module_arc = Arc::new(module);
            self.modules.insert(name.to_string(), Arc::clone(&module_arc));
            return Ok(module_arc);
        }

        // Try to find the module in sys.path
        for path in self.sys_path.iter() {
            // Check for package (directory with __init__.py)
            let package_dir = path.join(name);
            let init_py = package_dir.join("__init__.py");
            if init_py.exists() {
                let module = self.load_source_module(name, &init_py, true)?;
                let module_arc = Arc::new(module);
                self.modules.insert(name.to_string(), Arc::clone(&module_arc));
                return Ok(module_arc);
            }

            // Check for source file
            let py_file = path.join(format!("{}.py", name));
            if py_file.exists() {
                let module = self.load_source_module(name, &py_file, false)?;
                let module_arc = Arc::new(module);
                self.modules.insert(name.to_string(), Arc::clone(&module_arc));
                return Ok(module_arc);
            }

            // Check for DPM binary module
            let dpm_file = path.join(format!("{}.dpm", name));
            if dpm_file.exists() {
                // DPM loading not yet implemented
                continue;
            }
        }

        Err(InterpreterError::ImportError(format!("No module named '{}'", name)))
    }

    /// Create a built-in module with standard attributes
    fn create_builtin_module(&self, name: &str) -> InterpreterResult<PyModule> {
        let mut module = PyModule::new(name);

        // Set standard module attributes
        module.dict.insert("__name__".to_string(), PyValue::Str(Arc::from(name)));
        module.dict.insert("__doc__".to_string(), PyValue::None);
        module.dict.insert("__package__".to_string(), PyValue::Str(Arc::from("")));
        module
            .dict
            .insert("__loader__".to_string(), PyValue::Str(Arc::from("<built-in>")));
        module.dict.insert("__spec__".to_string(), PyValue::None);

        // Add module-specific attributes based on the module name
        match name {
            "sys" => {
                // Add sys module attributes
                module
                    .dict
                    .insert("version".to_string(), PyValue::Str(Arc::from("3.12.0 (dx-py)")));
                module
                    .dict
                    .insert("platform".to_string(), PyValue::Str(Arc::from(std::env::consts::OS)));
                module.dict.insert(
                    "executable".to_string(),
                    PyValue::Str(Arc::from(
                        std::env::current_exe()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    )),
                );
                // sys.path as a list
                let path_list: Vec<PyValue> = self
                    .sys_path
                    .iter()
                    .map(|p| PyValue::Str(Arc::from(p.to_string_lossy().as_ref())))
                    .collect();
                module.dict.insert(
                    "path".to_string(),
                    PyValue::List(Arc::new(dx_py_core::PyList::from_values(path_list))),
                );
                // sys.modules reference (simplified)
                module
                    .dict
                    .insert("modules".to_string(), PyValue::Dict(Arc::new(PyDict::new())));
            }
            "os" => {
                // Add os module attributes
                module.dict.insert(
                    "name".to_string(),
                    PyValue::Str(Arc::from(if cfg!(windows) { "nt" } else { "posix" })),
                );
                module.dict.insert(
                    "sep".to_string(),
                    PyValue::Str(Arc::from(std::path::MAIN_SEPARATOR.to_string())),
                );
                module.dict.insert(
                    "linesep".to_string(),
                    PyValue::Str(Arc::from(if cfg!(windows) { "\r\n" } else { "\n" })),
                );
                // Add getcwd as a builtin function if available
                if let Some(cwd) = self.builtins.get("os.getcwd") {
                    module.dict.insert("getcwd".to_string(), PyValue::Builtin(Arc::clone(cwd)));
                }
                
                // Add os.path as a submodule
                let path_module = self.create_builtin_module("os.path")?;
                module.dict.insert("path".to_string(), PyValue::Module(Arc::new(path_module)));
            }
            "os.path" => {
                // Add os.path module functions from stdlib
                for builtin in dx_py_core::stdlib::os_path_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "math" => {
                // Add math constants
                module.dict.insert("pi".to_string(), PyValue::Float(std::f64::consts::PI));
                module.dict.insert("e".to_string(), PyValue::Float(std::f64::consts::E));
                module.dict.insert("tau".to_string(), PyValue::Float(std::f64::consts::TAU));
                module.dict.insert("inf".to_string(), PyValue::Float(f64::INFINITY));
                module.dict.insert("nan".to_string(), PyValue::Float(f64::NAN));
            }
            "json" => {
                // Add json module functions from stdlib
                for builtin in dx_py_core::stdlib::json_builtins_expanded() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "collections" => {
                // Add collections module functions from stdlib
                for builtin in dx_py_core::stdlib::collections_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "itertools" => {
                // Add itertools module functions from stdlib
                for builtin in dx_py_core::stdlib::itertools_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "functools" => {
                // Add functools module functions from stdlib
                for builtin in dx_py_core::stdlib::functools_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "re" => {
                // Add re module functions from stdlib
                for builtin in dx_py_core::stdlib::re_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "datetime" => {
                // Add datetime module functions from stdlib
                for builtin in dx_py_core::stdlib::datetime_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "pathlib" => {
                // Add pathlib module functions from stdlib
                for builtin in dx_py_core::stdlib::pathlib_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "typing" => {
                // Typing module - mostly used for type hints
            }
            _ => {
                // Other built-in modules get basic attributes only
            }
        }

        module.mark_initialized();
        Ok(module)
    }

    /// Load a Python source module from a file
    fn load_source_module(
        &self,
        name: &str,
        path: &std::path::Path,
        is_package: bool,
    ) -> InterpreterResult<PyModule> {
        // Read the source file
        let source = std::fs::read_to_string(path).map_err(|e| {
            InterpreterError::ImportError(format!("Cannot read '{}': {}", path.display(), e))
        })?;

        let mut module = PyModule::new(name).with_file(path.to_path_buf());

        // Set package attribute
        if is_package {
            module = module.with_package(name);
            // Set __path__ for packages
            if let Some(parent) = path.parent() {
                module.dict.insert(
                    "__path__".to_string(),
                    PyValue::List(Arc::new(dx_py_core::PyList::from_values(vec![PyValue::Str(
                        Arc::from(parent.to_string_lossy().as_ref()),
                    )]))),
                );
            }
        }

        // Extract module docstring
        if let Some(doc) = Self::extract_docstring(&source) {
            module = module.with_doc(doc);
        }

        // Extract __all__ if defined
        if let Some(all_list) = Self::extract_all(&source) {
            let all_values: Vec<PyValue> =
                all_list.into_iter().map(|s| PyValue::Str(Arc::from(s))).collect();
            module.dict.insert(
                "__all__".to_string(),
                PyValue::List(Arc::new(dx_py_core::PyList::from_values(all_values))),
            );
        }

        // Extract simple definitions from source
        Self::extract_definitions(&source, &module);

        module.mark_initialized();
        Ok(module)
    }

    /// Extract module docstring from source
    fn extract_docstring(source: &str) -> Option<String> {
        let trimmed = source.trim_start();

        // Check for triple-quoted string at start
        if let Some(rest) = trimmed.strip_prefix("\"\"\"") {
            if let Some(end) = rest.find("\"\"\"") {
                return Some(rest[..end].trim().to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("'''") {
            if let Some(end) = rest.find("'''") {
                return Some(rest[..end].trim().to_string());
            }
        }

        None
    }

    /// Extract __all__ list from source
    fn extract_all(source: &str) -> Option<Vec<String>> {
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("__all__") {
                if let Some(eq_pos) = trimmed.find('=') {
                    let value = trimmed[eq_pos + 1..].trim();
                    if value.starts_with('[') && value.ends_with(']') {
                        let inner = &value[1..value.len() - 1];
                        let names: Vec<String> = inner
                            .split(',')
                            .filter_map(|s| {
                                let s = s.trim();
                                if (s.starts_with('\'') && s.ends_with('\''))
                                    || (s.starts_with('"') && s.ends_with('"'))
                                {
                                    Some(s[1..s.len() - 1].to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !names.is_empty() {
                            return Some(names);
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract simple definitions from source (functions, classes, constants)
    fn extract_definitions(source: &str, module: &PyModule) {
        for line in source.lines() {
            let trimmed = line.trim();

            // Function definition at module level (no leading whitespace)
            if line.starts_with("def ") {
                if let Some(name) = trimmed.strip_prefix("def ") {
                    let name = name.split('(').next().unwrap_or("").trim();
                    if !name.is_empty() {
                        module.set_attr(
                            name,
                            PyValue::Str(Arc::from(format!("<function {}>", name))),
                        );
                    }
                }
            }
            // Class definition at module level
            else if line.starts_with("class ") {
                if let Some(name) = trimmed.strip_prefix("class ") {
                    let name = name.split(['(', ':']).next().unwrap_or("").trim();
                    if !name.is_empty() {
                        module.set_attr(name, PyValue::Str(Arc::from(format!("<class {}>", name))));
                    }
                }
            }
            // Module-level assignment (simple cases)
            else if !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('#') {
                if let Some(eq_pos) = line.find('=') {
                    let before_eq = &line[..eq_pos];
                    if !before_eq.ends_with(['!', '<', '>', '+', '-', '*', '/', '%', '&', '|', '^'])
                    {
                        let name = before_eq.trim();
                        if !name.is_empty()
                            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                            && !name.starts_with(char::is_numeric)
                        {
                            let value = line[eq_pos + 1..].trim();
                            if let Some(v) = Self::parse_simple_value(value) {
                                module.set_attr(name, v);
                            } else {
                                module.set_attr(name, PyValue::Str(Arc::from(value)));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Parse simple Python values
    fn parse_simple_value(value: &str) -> Option<PyValue> {
        let value = value.trim();

        if value == "None" {
            return Some(PyValue::None);
        }
        if value == "True" {
            return Some(PyValue::Bool(true));
        }
        if value == "False" {
            return Some(PyValue::Bool(false));
        }
        if let Ok(i) = value.parse::<i64>() {
            return Some(PyValue::Int(i));
        }
        if let Ok(f) = value.parse::<f64>() {
            return Some(PyValue::Float(f));
        }
        if (value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"'))
        {
            return Some(PyValue::Str(Arc::from(&value[1..value.len() - 1])));
        }
        if (value.starts_with("'''") && value.ends_with("'''"))
            || (value.starts_with("\"\"\"") && value.ends_with("\"\"\""))
        {
            return Some(PyValue::Str(Arc::from(&value[3..value.len() - 3])));
        }

        None
    }

    /// Handle __build_class__ builtin call specially
    /// This executes the class body function to populate the class namespace
    fn handle_build_class(&self, args: &[PyValue]) -> InterpreterResult<PyValue> {
        // __build_class__(class_body_func, class_name, *bases, **kwargs)
        if args.len() < 2 {
            return Err(InterpreterError::TypeError(
                "at least 2 arguments required".into(),
            ));
        }

        // First argument is the class body function
        let class_body_func = &args[0];

        // Second argument is the class name
        let class_name = match &args[1] {
            PyValue::Str(s) => s.to_string(),
            _ => {
                return Err(InterpreterError::TypeError(
                    "class name must be a string".into(),
                ))
            }
        };

        // Remaining arguments are base classes
        let mut bases: Vec<Arc<dx_py_core::types::PyType>> = Vec::new();
        for arg in args.iter().skip(2) {
            match arg {
                PyValue::Type(t) => bases.push(Arc::clone(t)),
                _ => {
                    // For now, skip non-type bases (could be metaclass keyword args)
                }
            }
        }

        // Create the class object with computed MRO
        let class = if bases.is_empty() {
            dx_py_core::types::PyType::new(&class_name)
        } else {
            dx_py_core::types::PyType::with_bases(&class_name, bases)
        };

        // Execute class body function to populate namespace
        if let PyValue::Function(func) = class_body_func {
            // Create a temporary globals dict that will capture class attributes
            let class_globals = Arc::new(dx_py_core::PyDict::new());
            
            // Create a new dispatcher with the class globals
            let class_dispatcher = Dispatcher::with_globals(
                self.code.clone(),
                self.constants.clone(),
                self.names.clone(),
                Arc::clone(&class_globals),
                self.builtins.clone(),
            );
            
            // Execute the class body function
            match class_dispatcher.call_user_function(func, &[]) {
                Ok(_) => {
                    // Copy all items from class_globals to the class dict
                    for (key, value) in class_globals.items() {
                        let key_str = match &key {
                            dx_py_core::pydict::PyKey::Str(s) => s.to_string(),
                            dx_py_core::pydict::PyKey::Int(i) => i.to_string(),
                            dx_py_core::pydict::PyKey::Bool(b) => b.to_string(),
                            dx_py_core::pydict::PyKey::None => "None".to_string(),
                            dx_py_core::pydict::PyKey::Tuple(_) => format!("{:?}", key),
                        };
                        class.dict.insert(key_str, value);
                    }
                }
                Err(_) => {
                    // Ignore errors from class body execution for now
                    // The class is still created
                }
            }
        }

        // Return the class
        Ok(PyValue::Type(Arc::new(class)))
    }

    /// Handle the builtin next() function
    /// This needs special handling because next() on generators requires executing bytecode
    fn handle_builtin_next(&self, args: &[PyValue]) -> InterpreterResult<PyValue> {
        if args.is_empty() {
            return Err(InterpreterError::TypeError(
                "next expected at least 1 argument, got 0".into(),
            ));
        }

        let iterator = &args[0];
        let default = args.get(1);

        match iterator {
            PyValue::Generator(gen) => {
                match self.execute_generator_next(gen)? {
                    Some(value) => Ok(value),
                    None => {
                        // Generator exhausted
                        match default {
                            Some(d) => Ok(d.clone()),
                            None => Err(InterpreterError::Exception(PyValue::Str(Arc::from(
                                "StopIteration",
                            )))),
                        }
                    }
                }
            }
            PyValue::Iterator(it) => {
                match it.next() {
                    Some(value) => Ok(value),
                    None => {
                        match default {
                            Some(d) => Ok(d.clone()),
                            None => Err(InterpreterError::Exception(PyValue::Str(Arc::from(
                                "StopIteration",
                            )))),
                        }
                    }
                }
            }
            _ => Err(InterpreterError::TypeError(format!(
                "'{}' object is not an iterator",
                iterator.type_name()
            ))),
        }
    }

    /// Execute a generator to get the next value
    /// Returns Some(value) if the generator yielded, None if exhausted (StopIteration)
    fn execute_generator_next(&self, gen: &Arc<PyGenerator>) -> InterpreterResult<Option<PyValue>> {
        use dx_py_core::pygenerator::GeneratorState;

        // Check generator state
        let state = gen.get_state();
        match state {
            GeneratorState::Completed | GeneratorState::Failed => {
                // Generator is exhausted
                return Ok(None);
            }
            GeneratorState::Running => {
                return Err(InterpreterError::Runtime(
                    "generator already executing".into(),
                ));
            }
            GeneratorState::Created | GeneratorState::Suspended => {
                // Continue execution
            }
        }

        // Get the frame from the generator
        let mut frame = match gen.get_frame() {
            Some(f) => f,
            None => {
                // No frame means generator is exhausted
                gen.complete(PyValue::None);
                return Ok(None);
            }
        };

        // If resuming from suspended state, push the sent value onto the stack
        // This is what the generator receives from yield (None for next())
        if state == GeneratorState::Suspended {
            // Get the sent value (None for next(), or the value passed to send())
            let sent_value = gen.take_send_value().unwrap_or(PyValue::None);
            frame.push(sent_value);
        }

        // Get the function code
        let (bytecode, constants, names) = self.get_function_code(&gen.function)?;

        // Create a dispatcher for the generator's bytecode
        let gen_dispatcher = Dispatcher::with_globals(
            bytecode,
            constants,
            names,
            Arc::clone(&self.globals),
            self.builtins.clone(),
        );

        // Mark generator as running
        *gen.state.lock() = GeneratorState::Running;

        // Execute until yield or return
        match gen_dispatcher.execute(&mut frame) {
            Ok(value) => {
                // Check if this was a yield or a return
                // If the frame's IP is at the end or we got a return, the generator is done
                if frame.ip >= gen_dispatcher.code.len() {
                    // Generator completed
                    gen.complete(value.clone());
                    Ok(None)
                } else {
                    // Generator yielded - save the frame and return the value
                    gen.yield_value(value.clone());
                    gen.set_frame(frame);
                    Ok(Some(value))
                }
            }
            Err(InterpreterError::Exception(exc)) => {
                // Generator raised an exception
                gen.fail();
                Err(InterpreterError::Exception(exc))
            }
            Err(e) => {
                gen.fail();
                Err(e)
            }
        }
    }

    /// Execute a coroutine to get the next value
    /// Returns Some(value) if the coroutine yielded (awaiting), None if completed
    fn execute_coroutine_next(&self, coro: &Arc<PyCoroutine>) -> InterpreterResult<Option<PyValue>> {
        use dx_py_core::pygenerator::CoroutineState;

        // Check coroutine state
        let state = coro.get_state();
        match state {
            CoroutineState::Completed | CoroutineState::Failed => {
                // Coroutine is done
                return Ok(None);
            }
            CoroutineState::Running => {
                return Err(InterpreterError::Runtime(
                    "coroutine already executing".into(),
                ));
            }
            CoroutineState::Created | CoroutineState::Suspended => {
                // Continue execution
            }
        }

        // Get the frame from the coroutine
        let mut frame = match coro.get_frame() {
            Some(f) => f,
            None => {
                // No frame means coroutine is done
                coro.complete();
                return Ok(None);
            }
        };

        // If resuming from suspended state, push the sent value onto the stack
        if state == CoroutineState::Suspended {
            let sent_value = coro.take_send_value().unwrap_or(PyValue::None);
            frame.push(sent_value);
        }

        // Get the function code
        let (bytecode, constants, names) = self.get_function_code(&coro.function)?;

        // Create a dispatcher for the coroutine's bytecode
        let coro_dispatcher = Dispatcher::with_globals(
            bytecode,
            constants,
            names,
            Arc::clone(&self.globals),
            self.builtins.clone(),
        );

        // Mark coroutine as running
        *coro.state.lock() = CoroutineState::Running;

        // Execute until await or return
        match coro_dispatcher.execute(&mut frame) {
            Ok(value) => {
                // Check if this was an await or a return
                if frame.ip >= coro_dispatcher.code.len() {
                    // Coroutine completed
                    coro.complete();
                    Ok(None)
                } else {
                    // Coroutine suspended (awaiting) - save the frame and return the value
                    coro.suspend();
                    coro.set_frame(frame);
                    Ok(Some(value))
                }
            }
            Err(InterpreterError::Exception(exc)) => {
                // Coroutine raised an exception
                coro.fail();
                Err(InterpreterError::Exception(exc))
            }
            Err(e) => {
                coro.fail();
                Err(e)
            }
        }
    }
}
