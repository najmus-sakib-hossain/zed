//! Built-in functions for DX-Py runtime

use crate::pyfunction::PyBuiltinFunction;
use crate::pylist::PyValue;
use crate::{RuntimeError, RuntimeResult};
use std::sync::Arc;

/// Create the print builtin
pub fn builtin_print() -> PyBuiltinFunction {
    PyBuiltinFunction::new("print", |args| {
        let output: Vec<String> = args.iter().map(format_value).collect();
        println!("{}", output.join(" "));
        Ok(PyValue::None)
    })
}

/// Create the len builtin
pub fn builtin_len() -> PyBuiltinFunction {
    PyBuiltinFunction::new("len", |args| match args.first() {
        Some(PyValue::Str(s)) => Ok(PyValue::Int(s.chars().count() as i64)),
        Some(PyValue::List(l)) => Ok(PyValue::Int(l.len() as i64)),
        Some(PyValue::Tuple(t)) => Ok(PyValue::Int(t.len() as i64)),
        Some(PyValue::Dict(d)) => Ok(PyValue::Int(d.len() as i64)),
        Some(v) => Err(RuntimeError::type_error("sized", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the type builtin
pub fn builtin_type() -> PyBuiltinFunction {
    PyBuiltinFunction::new("type", |args| match args.first() {
        Some(v) => Ok(PyValue::Str(Arc::from(v.type_name()))),
        None => Err(RuntimeError::type_error("1 or 3 arguments", "0 arguments")),
    })
}

/// Create the int builtin
pub fn builtin_int() -> PyBuiltinFunction {
    PyBuiltinFunction::new("int", |args| match args.first() {
        Some(PyValue::Int(i)) => Ok(PyValue::Int(*i)),
        Some(PyValue::Float(f)) => Ok(PyValue::Int(*f as i64)),
        Some(PyValue::Bool(b)) => Ok(PyValue::Int(*b as i64)),
        Some(PyValue::Str(s)) => s
            .parse::<i64>()
            .map(PyValue::Int)
            .map_err(|_| RuntimeError::value_error(format!("invalid literal for int(): '{}'", s))),
        Some(v) => Err(RuntimeError::type_error("string or number", v.type_name())),
        None => Ok(PyValue::Int(0)),
    })
}

/// Create the float builtin
pub fn builtin_float() -> PyBuiltinFunction {
    PyBuiltinFunction::new("float", |args| match args.first() {
        Some(PyValue::Float(f)) => Ok(PyValue::Float(*f)),
        Some(PyValue::Int(i)) => Ok(PyValue::Float(*i as f64)),
        Some(PyValue::Bool(b)) => Ok(PyValue::Float(*b as i64 as f64)),
        Some(PyValue::Str(s)) => s.parse::<f64>().map(PyValue::Float).map_err(|_| {
            RuntimeError::value_error(format!("could not convert string to float: '{}'", s))
        }),
        Some(v) => Err(RuntimeError::type_error("string or number", v.type_name())),
        None => Ok(PyValue::Float(0.0)),
    })
}

/// Create the str builtin
pub fn builtin_str() -> PyBuiltinFunction {
    PyBuiltinFunction::new("str", |args| match args.first() {
        Some(v) => Ok(PyValue::Str(Arc::from(format_value(v)))),
        None => Ok(PyValue::Str(Arc::from(""))),
    })
}

/// Create the bool builtin
pub fn builtin_bool() -> PyBuiltinFunction {
    PyBuiltinFunction::new("bool", |args| match args.first() {
        Some(v) => Ok(PyValue::Bool(v.to_bool())),
        None => Ok(PyValue::Bool(false)),
    })
}

/// Create the abs builtin
pub fn builtin_abs() -> PyBuiltinFunction {
    PyBuiltinFunction::new("abs", |args| match args.first() {
        Some(PyValue::Int(i)) => Ok(PyValue::Int(i.abs())),
        Some(PyValue::Float(f)) => Ok(PyValue::Float(f.abs())),
        Some(v) => Err(RuntimeError::type_error("number", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the min builtin
pub fn builtin_min() -> PyBuiltinFunction {
    PyBuiltinFunction::new("min", |args| {
        if args.is_empty() {
            return Err(RuntimeError::type_error("at least 1 argument", "0 arguments"));
        }

        let mut min_val = args[0].clone();
        for arg in &args[1..] {
            if compare_values(arg, &min_val)? < 0 {
                min_val = arg.clone();
            }
        }
        Ok(min_val)
    })
}

/// Create the max builtin
pub fn builtin_max() -> PyBuiltinFunction {
    PyBuiltinFunction::new("max", |args| {
        if args.is_empty() {
            return Err(RuntimeError::type_error("at least 1 argument", "0 arguments"));
        }

        let mut max_val = args[0].clone();
        for arg in &args[1..] {
            if compare_values(arg, &max_val)? > 0 {
                max_val = arg.clone();
            }
        }
        Ok(max_val)
    })
}

/// Create the sum builtin
pub fn builtin_sum() -> PyBuiltinFunction {
    PyBuiltinFunction::new("sum", |args| match args.first() {
        Some(PyValue::List(list)) => {
            let mut total: i64 = 0;
            for item in list.to_vec() {
                match item {
                    PyValue::Int(i) => total += i,
                    _ => return Err(RuntimeError::type_error("number", item.type_name())),
                }
            }
            Ok(PyValue::Int(total))
        }
        _ => Err(RuntimeError::type_error("iterable", "non-iterable")),
    })
}

/// Create the range builtin (returns a list for simplicity)
pub fn builtin_range() -> PyBuiltinFunction {
    PyBuiltinFunction::new("range", |args| {
        let (start, stop, step) = match args.len() {
            1 => match &args[0] {
                PyValue::Int(stop) => (0, *stop, 1),
                _ => return Err(RuntimeError::type_error("int", args[0].type_name())),
            },
            2 => match (&args[0], &args[1]) {
                (PyValue::Int(start), PyValue::Int(stop)) => (*start, *stop, 1),
                _ => return Err(RuntimeError::type_error("int", "non-int")),
            },
            3 => match (&args[0], &args[1], &args[2]) {
                (PyValue::Int(start), PyValue::Int(stop), PyValue::Int(step)) => {
                    if *step == 0 {
                        return Err(RuntimeError::value_error("range() step cannot be zero"));
                    }
                    (*start, *stop, *step)
                }
                _ => return Err(RuntimeError::type_error("int", "non-int")),
            },
            _ => {
                return Err(RuntimeError::type_error(
                    "at most 3 arguments",
                    format!("{} arguments", args.len()),
                ))
            }
        };

        let mut result = Vec::new();
        let mut i = start;
        if step > 0 {
            while i < stop {
                result.push(PyValue::Int(i));
                i += step;
            }
        } else {
            while i > stop {
                result.push(PyValue::Int(i));
                i += step;
            }
        }

        Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
    })
}

/// Format a value for display
fn format_value(value: &PyValue) -> String {
    match value {
        PyValue::None => "None".to_string(),
        PyValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        PyValue::Int(i) => i.to_string(),
        PyValue::Float(f) => format!("{}", f),
        PyValue::Str(s) => s.to_string(),
        PyValue::List(l) => {
            let items: Vec<String> = l.to_vec().iter().map(repr_value).collect();
            format!("[{}]", items.join(", "))
        }
        PyValue::Set(s) => {
            let items: Vec<String> = s.to_vec().iter().map(repr_value).collect();
            if items.is_empty() {
                "set()".to_string()
            } else {
                format!("{{{}}}", items.join(", "))
            }
        }
        PyValue::Tuple(t) => {
            let items: Vec<String> = t.to_vec().iter().map(repr_value).collect();
            if items.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        PyValue::Dict(d) => {
            let items: Vec<String> = d
                .items()
                .iter()
                .map(|(k, v)| format!("{}: {}", format!("{:?}", k), repr_value(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        PyValue::Exception(e) => format!("{}: {}", e.exc_type, e.message),
        PyValue::Type(t) => format!("<class '{}'>", t.name),
        PyValue::Instance(inst) => format!("<{} object>", inst.class.name),
        PyValue::BoundMethod(method) => match method {
            crate::types::BoundMethod::Instance { .. } => "<bound method>".to_string(),
            crate::types::BoundMethod::Class { .. } => "<bound class method>".to_string(),
            crate::types::BoundMethod::Static { .. } => "<static method>".to_string(),
            crate::types::BoundMethod::Unbound { .. } => "<unbound method>".to_string(),
            crate::types::BoundMethod::String { method, .. } => format!("<built-in method {} of str>", method),
            crate::types::BoundMethod::List { method, .. } => format!("<built-in method {} of list>", method),
            crate::types::BoundMethod::Dict { method, .. } => format!("<built-in method {} of dict>", method),
        },
        PyValue::Generator(gen) => format!("<generator object {} at {:p}>", gen.name, gen.as_ref()),
        PyValue::Coroutine(coro) => {
            format!("<coroutine object {} at {:p}>", coro.name, coro.as_ref())
        }
        PyValue::Builtin(b) => format!("<built-in function {}>", b.name),
        PyValue::Function(f) => format!("<function {}>", f.name),
        PyValue::Iterator(_) => "<iterator>".to_string(),
        PyValue::Module(m) => format!("<module '{}'>", m.name),
        PyValue::Code(c) => format!("<code object {}>", c.name),
        PyValue::Cell(cell) => format!("<cell: {:?}>", cell.get()),
        PyValue::Super(s) => format!(
            "<super: <class '{}'>, {}>",
            s.type_.name,
            s.obj
                .as_ref()
                .map(|o| format!("<{} object>", o.class.name))
                .unwrap_or_else(|| "NULL".to_string())
        ),
        PyValue::Property(p) => format!("<property object: {}>", p.get_doc().unwrap_or("no doc")),
        PyValue::StaticMethod(f) => format!("<staticmethod({})>", format_value(f)),
        PyValue::ClassMethod(f) => format!("<classmethod({})>", format_value(f)),
    }
}

/// Repr a value (with quotes for strings)
fn repr_value(value: &PyValue) -> String {
    match value {
        PyValue::Str(s) => format!("'{}'", s),
        _ => format_value(value),
    }
}

/// Compare two values, returns -1, 0, or 1
fn compare_values(a: &PyValue, b: &PyValue) -> RuntimeResult<i32> {
    match (a, b) {
        (PyValue::Int(x), PyValue::Int(y)) => Ok(x.cmp(y) as i32),
        (PyValue::Float(x), PyValue::Float(y)) => {
            Ok(x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal) as i32)
        }
        (PyValue::Int(x), PyValue::Float(y)) => {
            Ok((*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal) as i32)
        }
        (PyValue::Float(x), PyValue::Int(y)) => {
            Ok(x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal) as i32)
        }
        (PyValue::Str(x), PyValue::Str(y)) => Ok(x.cmp(y) as i32),
        _ => Err(RuntimeError::type_error(a.type_name(), b.type_name())),
    }
}

// ===== Type Constructors (Task 7.1) =====

/// Create the list builtin
pub fn builtin_list() -> PyBuiltinFunction {
    PyBuiltinFunction::new("list", |args| match args.first() {
        Some(PyValue::List(l)) => Ok(PyValue::List(Arc::clone(l))),
        Some(PyValue::Tuple(t)) => {
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(t.to_vec()))))
        }
        Some(PyValue::Str(s)) => {
            let chars: Vec<PyValue> =
                s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect();
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(chars))))
        }
        Some(v) => Err(RuntimeError::type_error("iterable", v.type_name())),
        None => Ok(PyValue::List(Arc::new(crate::PyList::new()))),
    })
}

/// Create the dict builtin
pub fn builtin_dict() -> PyBuiltinFunction {
    PyBuiltinFunction::new("dict", |args| match args.first() {
        Some(PyValue::Dict(d)) => Ok(PyValue::Dict(Arc::clone(d))),
        Some(v) => Err(RuntimeError::type_error("mapping or iterable", v.type_name())),
        None => Ok(PyValue::Dict(Arc::new(crate::PyDict::new()))),
    })
}

/// Create the set builtin (returns a list for now since we don't have PySet)
pub fn builtin_set() -> PyBuiltinFunction {
    PyBuiltinFunction::new("set", |args| match args.first() {
        Some(PyValue::List(l)) => {
            // Deduplicate - simplified version
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            for item in l.to_vec() {
                let key = format!("{:?}", item);
                if !seen.contains(&key) {
                    seen.insert(key);
                    result.push(item);
                }
            }
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
        }
        Some(PyValue::Str(s)) => {
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            for c in s.chars() {
                if !seen.contains(&c) {
                    seen.insert(c);
                    result.push(PyValue::Str(Arc::from(c.to_string())));
                }
            }
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
        }
        Some(v) => Err(RuntimeError::type_error("iterable", v.type_name())),
        None => Ok(PyValue::List(Arc::new(crate::PyList::new()))),
    })
}

/// Create the tuple builtin
pub fn builtin_tuple() -> PyBuiltinFunction {
    PyBuiltinFunction::new("tuple", |args| match args.first() {
        Some(PyValue::Tuple(t)) => Ok(PyValue::Tuple(Arc::clone(t))),
        Some(PyValue::List(l)) => {
            Ok(PyValue::Tuple(Arc::new(crate::PyTuple::from_values(l.to_vec()))))
        }
        Some(PyValue::Str(s)) => {
            let chars: Vec<PyValue> =
                s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect();
            Ok(PyValue::Tuple(Arc::new(crate::PyTuple::from_values(chars))))
        }
        Some(v) => Err(RuntimeError::type_error("iterable", v.type_name())),
        None => Ok(PyValue::Tuple(Arc::new(crate::PyTuple::empty()))),
    })
}

/// Create the bytes builtin
pub fn builtin_bytes() -> PyBuiltinFunction {
    PyBuiltinFunction::new("bytes", |args| match args.first() {
        Some(PyValue::Str(s)) => {
            // Convert string to list of byte values
            let bytes: Vec<PyValue> =
                s.as_bytes().iter().map(|b| PyValue::Int(*b as i64)).collect();
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(bytes))))
        }
        Some(PyValue::Int(n)) => {
            // Create n zero bytes
            let bytes: Vec<PyValue> = (0..*n).map(|_| PyValue::Int(0)).collect();
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(bytes))))
        }
        Some(PyValue::List(l)) => {
            // Validate all items are bytes (0-255)
            for item in l.to_vec() {
                match item {
                    PyValue::Int(i) if (0..=255).contains(&i) => {}
                    _ => return Err(RuntimeError::value_error("bytes must be in range(0, 256)")),
                }
            }
            Ok(PyValue::List(Arc::clone(l)))
        }
        Some(v) => Err(RuntimeError::type_error("string, int, or iterable", v.type_name())),
        None => Ok(PyValue::List(Arc::new(crate::PyList::new()))),
    })
}

/// Create the object builtin
pub fn builtin_object() -> PyBuiltinFunction {
    PyBuiltinFunction::new("object", |_args| {
        // Return a simple object representation
        Ok(PyValue::None) // Placeholder - real implementation would create an object
    })
}

// ===== Iteration Functions (Task 7.2) =====

/// Create the enumerate builtin
pub fn builtin_enumerate() -> PyBuiltinFunction {
    PyBuiltinFunction::new("enumerate", |args| {
        let iterable = match args.first() {
            Some(PyValue::List(l)) => l.to_vec(),
            Some(PyValue::Tuple(t)) => t.to_vec(),
            Some(PyValue::Str(s)) => {
                s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
            }
            Some(v) => return Err(RuntimeError::type_error("iterable", v.type_name())),
            None => return Err(RuntimeError::type_error("1 argument", "0 arguments")),
        };

        let start = match args.get(1) {
            Some(PyValue::Int(i)) => *i,
            Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            None => 0,
        };

        let result: Vec<PyValue> = iterable
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                PyValue::Tuple(Arc::new(crate::PyTuple::from_values(vec![
                    PyValue::Int(start + i as i64),
                    v,
                ])))
            })
            .collect();

        Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
    })
}

/// Create the zip builtin
pub fn builtin_zip() -> PyBuiltinFunction {
    PyBuiltinFunction::new("zip", |args| {
        if args.is_empty() {
            return Ok(PyValue::List(Arc::new(crate::PyList::new())));
        }

        // Convert all arguments to vectors
        let iterables: RuntimeResult<Vec<Vec<PyValue>>> = args
            .iter()
            .map(|arg| match arg {
                PyValue::List(l) => Ok(l.to_vec()),
                PyValue::Tuple(t) => Ok(t.to_vec()),
                PyValue::Str(s) => {
                    Ok(s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect())
                }
                v => Err(RuntimeError::type_error("iterable", v.type_name())),
            })
            .collect();
        let iterables = iterables?;

        // Find minimum length
        let min_len = iterables.iter().map(|v| v.len()).min().unwrap_or(0);

        // Create tuples
        let result: Vec<PyValue> = (0..min_len)
            .map(|i| {
                let tuple_items: Vec<PyValue> = iterables.iter().map(|v| v[i].clone()).collect();
                PyValue::Tuple(Arc::new(crate::PyTuple::from_values(tuple_items)))
            })
            .collect();

        Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
    })
}

/// Create the map builtin
/// map(function, iterable, ...) -> map object
///
/// Make an iterator that computes the function using arguments from
/// each of the iterables. Stops when the shortest iterable is exhausted.
///
/// For simplicity, this implementation returns a list instead of an iterator.
pub fn builtin_map() -> PyBuiltinFunction {
    PyBuiltinFunction::new("map", |args| {
        if args.len() < 2 {
            return Err(RuntimeError::type_error(
                "at least 2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let func = &args[0];

        // Validate that the first argument is callable
        match func {
            PyValue::Function(_) | PyValue::Builtin(_) | PyValue::BoundMethod(_) => {}
            _ => {
                return Err(RuntimeError::type_error("callable", func.type_name()));
            }
        }

        // Convert all iterables to vectors
        let iterables: RuntimeResult<Vec<Vec<PyValue>>> = args[1..]
            .iter()
            .map(|arg| match arg {
                PyValue::List(l) => Ok(l.to_vec()),
                PyValue::Tuple(t) => Ok(t.to_vec()),
                PyValue::Str(s) => {
                    Ok(s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect())
                }
                v => Err(RuntimeError::type_error("iterable", v.type_name())),
            })
            .collect();
        let iterables = iterables?;

        if iterables.is_empty() {
            return Ok(PyValue::List(Arc::new(crate::PyList::new())));
        }

        // Find minimum length
        let min_len = iterables.iter().map(|v| v.len()).min().unwrap_or(0);

        // Apply function to each set of arguments
        let mut result = Vec::with_capacity(min_len);
        for i in 0..min_len {
            let call_args: Vec<PyValue> = iterables.iter().map(|v| v[i].clone()).collect();

            // Call the function with the arguments
            let value = match func {
                PyValue::Builtin(b) => b.call(&call_args)?,
                PyValue::Function(_) | PyValue::BoundMethod(_) => {
                    // For user-defined functions, we need interpreter support
                    // For now, return a placeholder indicating the function would be called
                    // In a full implementation, this would invoke the VM
                    return Err(RuntimeError::internal_error(
                        "map() with user-defined functions requires interpreter support",
                    ));
                }
                _ => unreachable!(),
            };
            result.push(value);
        }

        Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
    })
}

/// Create the filter builtin
/// filter(function, iterable) -> filter object
///
/// Return an iterator yielding those items of iterable for which function(item)
/// is true. If function is None, return the items that are true.
///
/// For simplicity, this implementation returns a list instead of an iterator.
pub fn builtin_filter() -> PyBuiltinFunction {
    PyBuiltinFunction::new("filter", |args| {
        if args.len() != 2 {
            return Err(RuntimeError::type_error(
                "2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let func = &args[0];
        let iterable = match &args[1] {
            PyValue::List(l) => l.to_vec(),
            PyValue::Tuple(t) => t.to_vec(),
            PyValue::Str(s) => s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect(),
            v => return Err(RuntimeError::type_error("iterable", v.type_name())),
        };

        // If function is None, filter by truthiness
        if matches!(func, PyValue::None) {
            let result: Vec<PyValue> = iterable.into_iter().filter(|v| v.to_bool()).collect();
            return Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))));
        }

        // Validate that the first argument is callable
        match func {
            PyValue::Function(_) | PyValue::Builtin(_) | PyValue::BoundMethod(_) => {}
            _ => {
                return Err(RuntimeError::type_error("callable or None", func.type_name()));
            }
        }

        // Apply function to each item and filter
        let mut result = Vec::new();
        for item in iterable {
            let keep = match func {
                PyValue::Builtin(b) => {
                    let call_result = b.call(std::slice::from_ref(&item))?;
                    call_result.to_bool()
                }
                PyValue::Function(_) | PyValue::BoundMethod(_) => {
                    // For user-defined functions, we need interpreter support
                    // For now, return a placeholder indicating the function would be called
                    // In a full implementation, this would invoke the VM
                    return Err(RuntimeError::internal_error(
                        "filter() with user-defined functions requires interpreter support",
                    ));
                }
                _ => unreachable!(),
            };

            if keep {
                result.push(item);
            }
        }

        Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
    })
}

/// Create the sorted builtin
pub fn builtin_sorted() -> PyBuiltinFunction {
    PyBuiltinFunction::new("sorted", |args| {
        let iterable = match args.first() {
            Some(PyValue::List(l)) => l.to_vec(),
            Some(PyValue::Tuple(t)) => t.to_vec(),
            Some(v) => return Err(RuntimeError::type_error("iterable", v.type_name())),
            None => return Err(RuntimeError::type_error("1 argument", "0 arguments")),
        };

        let mut result = iterable;
        result.sort_by(|a, b| match compare_values(a, b) {
            Ok(c) if c < 0 => std::cmp::Ordering::Less,
            Ok(c) if c > 0 => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });

        Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
    })
}

/// Create the reversed builtin
pub fn builtin_reversed() -> PyBuiltinFunction {
    PyBuiltinFunction::new("reversed", |args| {
        let iterable = match args.first() {
            Some(PyValue::List(l)) => l.to_vec(),
            Some(PyValue::Tuple(t)) => t.to_vec(),
            Some(PyValue::Str(s)) => {
                s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
            }
            Some(v) => return Err(RuntimeError::type_error("sequence", v.type_name())),
            None => return Err(RuntimeError::type_error("1 argument", "0 arguments")),
        };

        let mut result = iterable;
        result.reverse();

        Ok(PyValue::List(Arc::new(crate::PyList::from_values(result))))
    })
}

/// Create the iter builtin
pub fn builtin_iter() -> PyBuiltinFunction {
    PyBuiltinFunction::new("iter", |args| match args.first() {
        Some(PyValue::List(l)) => Ok(PyValue::List(Arc::clone(l))),
        Some(PyValue::Tuple(t)) => {
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(t.to_vec()))))
        }
        Some(PyValue::Str(s)) => {
            let chars: Vec<PyValue> =
                s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect();
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(chars))))
        }
        Some(v) => Err(RuntimeError::type_error("iterable", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the next builtin
pub fn builtin_next() -> PyBuiltinFunction {
    PyBuiltinFunction::new("next", |args| {
        // Simplified - just returns first element or default
        match args.first() {
            Some(PyValue::List(l)) => {
                let items = l.to_vec();
                if !items.is_empty() {
                    Ok(items[0].clone())
                } else {
                    match args.get(1) {
                        Some(default) => Ok(default.clone()),
                        None => Err(RuntimeError::internal_error("StopIteration")),
                    }
                }
            }
            Some(v) => Err(RuntimeError::type_error("iterator", v.type_name())),
            None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
        }
    })
}

/// Create the all builtin
pub fn builtin_all() -> PyBuiltinFunction {
    PyBuiltinFunction::new("all", |args| {
        let iterable = match args.first() {
            Some(PyValue::List(l)) => l.to_vec(),
            Some(PyValue::Tuple(t)) => t.to_vec(),
            Some(v) => return Err(RuntimeError::type_error("iterable", v.type_name())),
            None => return Err(RuntimeError::type_error("1 argument", "0 arguments")),
        };

        for item in iterable {
            if !item.to_bool() {
                return Ok(PyValue::Bool(false));
            }
        }
        Ok(PyValue::Bool(true))
    })
}

/// Create the any builtin
pub fn builtin_any() -> PyBuiltinFunction {
    PyBuiltinFunction::new("any", |args| {
        let iterable = match args.first() {
            Some(PyValue::List(l)) => l.to_vec(),
            Some(PyValue::Tuple(t)) => t.to_vec(),
            Some(v) => return Err(RuntimeError::type_error("iterable", v.type_name())),
            None => return Err(RuntimeError::type_error("1 argument", "0 arguments")),
        };

        for item in iterable {
            if item.to_bool() {
                return Ok(PyValue::Bool(true));
            }
        }
        Ok(PyValue::Bool(false))
    })
}

// ===== Introspection Functions (Task 7.4) =====

/// Create the isinstance builtin
/// isinstance(object, classinfo) -> bool
/// Return whether an object is an instance of a class or of a subclass thereof.
/// A tuple, as in isinstance(x, (A, B, ...)), may be given as the target to check against.
pub fn builtin_isinstance() -> PyBuiltinFunction {
    PyBuiltinFunction::new("isinstance", |args| {
        if args.len() != 2 {
            return Err(RuntimeError::type_error(
                "2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let obj = &args[0];
        let classinfo = &args[1];

        // Helper function to check if obj is instance of a single type
        fn check_instance(obj: &PyValue, type_spec: &PyValue) -> RuntimeResult<bool> {
            match type_spec {
                // Check against a type object
                PyValue::Type(ty) => {
                    match obj {
                        PyValue::Instance(inst) => {
                            // Check if instance's class is the type or a subtype
                            if Arc::ptr_eq(&inst.class, ty) {
                                return Ok(true);
                            }
                            Ok(inst.class.is_subtype(ty))
                        }
                        // Check built-in types by name
                        _ => {
                            let obj_type_name = obj.type_name();
                            Ok(obj_type_name == ty.name)
                        }
                    }
                }
                // Check against a string type name (for convenience)
                PyValue::Str(type_name) => Ok(obj.type_name() == type_name.as_ref()),
                // Check against a tuple of types (any match)
                PyValue::Tuple(types) => {
                    for t in types.to_vec() {
                        if check_instance(obj, &t)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                _ => {
                    Err(RuntimeError::type_error("a type or tuple of types", type_spec.type_name()))
                }
            }
        }

        let result = check_instance(obj, classinfo)?;
        Ok(PyValue::Bool(result))
    })
}

/// Create the issubclass builtin
/// issubclass(class, classinfo) -> bool
/// Return whether 'class' is a derived from another class or is the same class.
/// A tuple, as in issubclass(x, (A, B, ...)), may be given as the target to check against.
pub fn builtin_issubclass() -> PyBuiltinFunction {
    PyBuiltinFunction::new("issubclass", |args| {
        if args.len() != 2 {
            return Err(RuntimeError::type_error(
                "2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let cls = &args[0];
        let classinfo = &args[1];

        // First argument must be a class
        let check_class = match cls {
            PyValue::Type(ty) => ty,
            _ => {
                return Err(RuntimeError::type_error("a class", cls.type_name()));
            }
        };

        // Helper function to check if cls is subclass of a single type
        fn check_subclass(
            cls: &Arc<crate::types::PyType>,
            type_spec: &PyValue,
        ) -> RuntimeResult<bool> {
            match type_spec {
                // Check against a type object
                PyValue::Type(ty) => {
                    if Arc::ptr_eq(cls, ty) {
                        return Ok(true);
                    }
                    Ok(cls.is_subtype(ty))
                }
                // Check against a string type name (for convenience)
                PyValue::Str(type_name) => {
                    if cls.name == type_name.as_ref() {
                        return Ok(true);
                    }
                    // Check MRO for name match
                    for base in &cls.mro {
                        if base.name == type_name.as_ref() {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                // Check against a tuple of types (any match)
                PyValue::Tuple(types) => {
                    for t in types.to_vec() {
                        if check_subclass(cls, &t)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                _ => {
                    Err(RuntimeError::type_error("a type or tuple of types", type_spec.type_name()))
                }
            }
        }

        let result = check_subclass(check_class, classinfo)?;
        Ok(PyValue::Bool(result))
    })
}

/// Create the super builtin
/// super() -> same as super(__class__, <first argument>)
/// super(type) -> unbound super object
/// super(type, obj) -> bound super object; requires isinstance(obj, type)
/// super(type, type2) -> bound super object; requires issubclass(type2, type)
///
/// Typical use to call a cooperative superclass method:
/// class C(B):
///     def method(self, arg):
///         super().method(arg)    # This does the same thing as:
///                                # super(C, self).method(arg)
pub fn builtin_super() -> PyBuiltinFunction {
    PyBuiltinFunction::new("super", |args| {
        use crate::types::PySuper;

        match args.len() {
            // super() - zero argument form
            // In a real implementation, this would use the implicit __class__ and first argument
            // For now, we return an error since we don't have access to the calling frame
            0 => Err(RuntimeError::type_error("super(): __class__ cell not found", "no arguments")),
            // super(type) - unbound super
            1 => {
                let type_ = match &args[0] {
                    PyValue::Type(ty) => Arc::clone(ty),
                    _ => {
                        return Err(RuntimeError::type_error("type", args[0].type_name()));
                    }
                };

                let super_obj = PySuper::new_with_types(type_, None, None);
                Ok(PyValue::Super(Arc::new(super_obj)))
            }
            // super(type, obj) or super(type, type2)
            2 => {
                let type_ = match &args[0] {
                    PyValue::Type(ty) => Arc::clone(ty),
                    _ => {
                        return Err(RuntimeError::type_error("type", args[0].type_name()));
                    }
                };

                match &args[1] {
                    // super(type, obj) - bound to instance
                    PyValue::Instance(inst) => {
                        // Check isinstance(obj, type)
                        if !Arc::ptr_eq(&inst.class, &type_) && !inst.class.is_subtype(&type_) {
                            return Err(RuntimeError::type_error(
                                format!("obj must be an instance or subtype of {}", type_.name),
                                inst.class.name.clone(),
                            ));
                        }

                        let super_obj = PySuper::new(Arc::clone(&type_), Some(Arc::clone(inst)));
                        Ok(PyValue::Super(Arc::new(super_obj)))
                    }
                    // super(type, type2) - bound to type
                    PyValue::Type(type2) => {
                        // Check issubclass(type2, type)
                        if !Arc::ptr_eq(type2, &type_) && !type2.is_subtype(&type_) {
                            return Err(RuntimeError::type_error(
                                format!("type2 must be a subtype of {}", type_.name),
                                type2.name.clone(),
                            ));
                        }

                        let super_obj = PySuper::new_with_types(
                            Arc::clone(&type_),
                            None,
                            Some(Arc::clone(type2)),
                        );
                        Ok(PyValue::Super(Arc::new(super_obj)))
                    }
                    _ => Err(RuntimeError::type_error("instance or type", args[1].type_name())),
                }
            }
            _ => {
                Err(RuntimeError::type_error("0-2 arguments", format!("{} arguments", args.len())))
            }
        }
    })
}

/// Create the hasattr builtin
/// hasattr(object, name) -> bool
/// Return whether the object has an attribute with the given name.
pub fn builtin_hasattr() -> PyBuiltinFunction {
    PyBuiltinFunction::new("hasattr", |args| {
        if args.len() != 2 {
            return Err(RuntimeError::type_error(
                "2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let obj = &args[0];
        let name = match &args[1] {
            PyValue::Str(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::type_error("str", args[1].type_name()));
            }
        };

        let has_attr = match obj {
            PyValue::Instance(inst) => inst.has_attr(name),
            PyValue::Type(ty) => ty.get_attr_from_mro(name).is_some(),
            PyValue::Module(m) => m.dict.contains_key(name),
            PyValue::Dict(d) => d.getitem(&crate::pydict::PyKey::Str(Arc::from(name))).is_ok(),
            // For built-in types, check common attributes
            _ => {
                // Check for common dunder methods
                matches!(name, "__class__" | "__doc__" | "__str__" | "__repr__" | "__hash__")
            }
        };

        Ok(PyValue::Bool(has_attr))
    })
}

/// Create the getattr builtin
/// getattr(object, name[, default]) -> value
/// Get a named attribute from an object; getattr(x, 'y') is equivalent to x.y.
/// When a default argument is given, it is returned when the attribute doesn't exist.
pub fn builtin_getattr() -> PyBuiltinFunction {
    PyBuiltinFunction::new("getattr", |args| {
        if args.len() < 2 || args.len() > 3 {
            return Err(RuntimeError::type_error(
                "2-3 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let obj = &args[0];
        let name = match &args[1] {
            PyValue::Str(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::type_error("str", args[1].type_name()));
            }
        };
        let default = args.get(2);

        let result = match obj {
            PyValue::Instance(inst) => inst.get_attr(name),
            PyValue::Type(ty) => ty.get_attr_from_mro(name),
            PyValue::Module(m) => m.dict.get(name).map(|v| v.clone()),
            PyValue::Dict(d) => d.getitem(&crate::pydict::PyKey::Str(Arc::from(name))).ok(),
            // Handle __class__ for all types
            _ if name == "__class__" => Some(PyValue::Str(Arc::from(obj.type_name()))),
            _ => None,
        };

        match result {
            Some(value) => Ok(value),
            None => match default {
                Some(d) => Ok(d.clone()),
                None => Err(RuntimeError::attribute_error(obj.type_name(), name)),
            },
        }
    })
}

/// Create the callable builtin
pub fn builtin_callable() -> PyBuiltinFunction {
    PyBuiltinFunction::new("callable", |args| match args.first() {
        Some(_) => Ok(PyValue::Bool(false)), // Simplified - would check for __call__
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the id builtin
pub fn builtin_id() -> PyBuiltinFunction {
    PyBuiltinFunction::new("id", |args| match args.first() {
        Some(v) => {
            // Return a hash-like value based on the value
            let id = match v {
                PyValue::Int(i) => *i as u64,
                PyValue::Float(f) => f.to_bits(),
                PyValue::Str(s) => {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    s.hash(&mut hasher);
                    hasher.finish()
                }
                _ => 0,
            };
            Ok(PyValue::Int(id as i64))
        }
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the hash builtin
pub fn builtin_hash() -> PyBuiltinFunction {
    PyBuiltinFunction::new("hash", |args| match args.first() {
        Some(PyValue::Int(i)) => Ok(PyValue::Int(*i)),
        Some(PyValue::Str(s)) => {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            s.hash(&mut hasher);
            Ok(PyValue::Int(hasher.finish() as i64))
        }
        Some(PyValue::Bool(b)) => Ok(PyValue::Int(*b as i64)),
        Some(PyValue::None) => Ok(PyValue::Int(0)),
        Some(v) => Err(RuntimeError::type_error("hashable", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the repr builtin
pub fn builtin_repr() -> PyBuiltinFunction {
    PyBuiltinFunction::new("repr", |args| match args.first() {
        Some(v) => Ok(PyValue::Str(Arc::from(repr_value(v)))),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the ord builtin
pub fn builtin_ord() -> PyBuiltinFunction {
    PyBuiltinFunction::new("ord", |args| match args.first() {
        Some(PyValue::Str(s)) => {
            let chars: Vec<char> = s.chars().collect();
            if chars.len() != 1 {
                return Err(RuntimeError::type_error(
                    "string of length 1",
                    format!("string of length {}", chars.len()),
                ));
            }
            Ok(PyValue::Int(chars[0] as i64))
        }
        Some(v) => Err(RuntimeError::type_error("string", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the chr builtin
pub fn builtin_chr() -> PyBuiltinFunction {
    PyBuiltinFunction::new("chr", |args| match args.first() {
        Some(PyValue::Int(i)) => {
            if *i < 0 || *i > 0x10FFFF {
                return Err(RuntimeError::value_error(format!("chr() arg not in range(0x110000)")));
            }
            match char::from_u32(*i as u32) {
                Some(c) => Ok(PyValue::Str(Arc::from(c.to_string()))),
                None => Err(RuntimeError::value_error("invalid Unicode code point")),
            }
        }
        Some(v) => Err(RuntimeError::type_error("int", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the hex builtin
pub fn builtin_hex() -> PyBuiltinFunction {
    PyBuiltinFunction::new("hex", |args| match args.first() {
        Some(PyValue::Int(i)) => {
            if *i < 0 {
                Ok(PyValue::Str(Arc::from(format!("-0x{:x}", -i))))
            } else {
                Ok(PyValue::Str(Arc::from(format!("0x{:x}", i))))
            }
        }
        Some(v) => Err(RuntimeError::type_error("int", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the bin builtin
pub fn builtin_bin() -> PyBuiltinFunction {
    PyBuiltinFunction::new("bin", |args| match args.first() {
        Some(PyValue::Int(i)) => {
            if *i < 0 {
                Ok(PyValue::Str(Arc::from(format!("-0b{:b}", -i))))
            } else {
                Ok(PyValue::Str(Arc::from(format!("0b{:b}", i))))
            }
        }
        Some(v) => Err(RuntimeError::type_error("int", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the oct builtin
pub fn builtin_oct() -> PyBuiltinFunction {
    PyBuiltinFunction::new("oct", |args| match args.first() {
        Some(PyValue::Int(i)) => {
            if *i < 0 {
                Ok(PyValue::Str(Arc::from(format!("-0o{:o}", -i))))
            } else {
                Ok(PyValue::Str(Arc::from(format!("0o{:o}", i))))
            }
        }
        Some(v) => Err(RuntimeError::type_error("int", v.type_name())),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the pow builtin
pub fn builtin_pow() -> PyBuiltinFunction {
    PyBuiltinFunction::new("pow", |args| {
        if args.len() < 2 {
            return Err(RuntimeError::type_error(
                "2-3 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let (base, exp) = match (&args[0], &args[1]) {
            (PyValue::Int(b), PyValue::Int(e)) => (*b, *e),
            (PyValue::Float(b), PyValue::Int(e)) => {
                return Ok(PyValue::Float(b.powi(*e as i32)));
            }
            (PyValue::Int(b), PyValue::Float(e)) => {
                return Ok(PyValue::Float((*b as f64).powf(*e)));
            }
            (PyValue::Float(b), PyValue::Float(e)) => {
                return Ok(PyValue::Float(b.powf(*e)));
            }
            _ => return Err(RuntimeError::type_error("number", "non-number")),
        };

        // Handle modulo if provided
        if let Some(PyValue::Int(m)) = args.get(2) {
            if *m == 0 {
                return Err(RuntimeError::value_error("pow() 3rd argument cannot be 0"));
            }
            // Modular exponentiation
            let mut result = 1i64;
            let mut base = base % m;
            let mut exp = exp;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result * base) % m;
                }
                exp /= 2;
                base = (base * base) % m;
            }
            return Ok(PyValue::Int(result));
        }

        if exp < 0 {
            Ok(PyValue::Float((base as f64).powi(exp as i32)))
        } else {
            Ok(PyValue::Int(base.pow(exp as u32)))
        }
    })
}

/// Create the round builtin
pub fn builtin_round() -> PyBuiltinFunction {
    PyBuiltinFunction::new("round", |args| {
        let number = match args.first() {
            Some(PyValue::Int(i)) => return Ok(PyValue::Int(*i)),
            Some(PyValue::Float(f)) => *f,
            Some(v) => return Err(RuntimeError::type_error("number", v.type_name())),
            None => return Err(RuntimeError::type_error("1-2 arguments", "0 arguments")),
        };

        let ndigits = match args.get(1) {
            Some(PyValue::Int(n)) => Some(*n),
            Some(PyValue::None) | None => None,
            Some(v) => return Err(RuntimeError::type_error("int or None", v.type_name())),
        };

        match ndigits {
            Some(n) => {
                let multiplier = 10f64.powi(n as i32);
                Ok(PyValue::Float((number * multiplier).round() / multiplier))
            }
            None => Ok(PyValue::Int(number.round() as i64)),
        }
    })
}

/// Create the divmod builtin
pub fn builtin_divmod() -> PyBuiltinFunction {
    PyBuiltinFunction::new("divmod", |args| {
        if args.len() != 2 {
            return Err(RuntimeError::type_error(
                "2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        match (&args[0], &args[1]) {
            (PyValue::Int(a), PyValue::Int(b)) => {
                if *b == 0 {
                    return Err(RuntimeError::value_error("integer division or modulo by zero"));
                }
                Ok(PyValue::Tuple(Arc::new(crate::PyTuple::from_values(vec![
                    PyValue::Int(a / b),
                    PyValue::Int(a % b),
                ]))))
            }
            (PyValue::Float(a), PyValue::Float(b)) => {
                if *b == 0.0 {
                    return Err(RuntimeError::value_error("float division by zero"));
                }
                Ok(PyValue::Tuple(Arc::new(crate::PyTuple::from_values(vec![
                    PyValue::Float((a / b).floor()),
                    PyValue::Float(a % b),
                ]))))
            }
            _ => Err(RuntimeError::type_error("numbers", "non-numbers")),
        }
    })
}

/// Create the property builtin
/// property(fget=None, fset=None, fdel=None, doc=None) -> property attribute
///
/// property() is a built-in function that returns a property attribute.
///
/// fget is a function to be used for getting an attribute value.
/// fset is a function to be used for setting an attribute value.
/// fdel is a function to be used for deleting an attribute.
/// doc is the docstring for the attribute.
///
/// A typical use is to define a managed attribute x:
///
/// ```text
/// class C:
///     def __init__(self):
///         self._x = None
///
///     def getx(self):
///         return self._x
///
///     def setx(self, value):
///         self._x = value
///
///     def delx(self):
///         del self._x
///
///     x = property(getx, setx, delx, "I'm the 'x' property.")
/// ```
pub fn builtin_property() -> PyBuiltinFunction {
    PyBuiltinFunction::new("property", |args| {
        use crate::types::PropertyDescriptor;

        let fget = args.get(0).cloned();
        let fset = args.get(1).cloned();
        let fdel = args.get(2).cloned();
        let doc = args.get(3).and_then(|v| match v {
            PyValue::Str(s) => Some(s.to_string()),
            _ => None,
        });

        let mut prop = PropertyDescriptor::new(fget, fset, fdel);
        if let Some(doc_str) = doc {
            prop = prop.with_doc(doc_str);
        }

        // Return the property descriptor as a PyValue::Property
        Ok(PyValue::Property(Arc::new(prop)))
    })
}

/// Create the staticmethod builtin
/// staticmethod(function) -> static method
///
/// Convert a function to be a static method.
///
/// A static method does not receive an implicit first argument.
/// To declare a static method, use this idiom:
///
/// ```text
///     class C:
///         @staticmethod
///         def f(arg1, arg2, ...):
///             ...
/// ```
///
/// It can be called either on the class (e.g. C.f()) or on an instance
/// (e.g. C().f()). Both the class and the instance are ignored, and
/// neither is passed implicitly as the first argument to the method.
pub fn builtin_staticmethod() -> PyBuiltinFunction {
    PyBuiltinFunction::new("staticmethod", |args| {
        if args.is_empty() {
            return Err(RuntimeError::type_error("1 argument", "0 arguments"));
        }

        let func = args[0].clone();

        // Validate that the argument is callable
        match &func {
            PyValue::Function(_) | PyValue::Builtin(_) | PyValue::BoundMethod(_) => {
                // Return a staticmethod descriptor wrapping the function
                Ok(PyValue::StaticMethod(Box::new(func)))
            }
            _ => {
                // Python allows any object to be wrapped in staticmethod
                // even if it's not callable (error happens at call time)
                Ok(PyValue::StaticMethod(Box::new(func)))
            }
        }
    })
}

/// Create the classmethod builtin
/// classmethod(function) -> class method
///
/// Convert a function to be a class method.
///
/// A class method receives the class as implicit first argument,
/// just like an instance method receives the instance.
/// To declare a class method, use this idiom:
///
/// ```text
///     class C:
///         @classmethod
///         def f(cls, arg1, arg2, ...):
///             ...
/// ```
///
/// It can be called either on the class (e.g. C.f()) or on an instance
/// (e.g. C().f()). The instance is ignored except for its class.
/// If a class method is called for a derived class, the derived class
/// object is passed as the implied first argument.
pub fn builtin_classmethod() -> PyBuiltinFunction {
    PyBuiltinFunction::new("classmethod", |args| {
        if args.is_empty() {
            return Err(RuntimeError::type_error("1 argument", "0 arguments"));
        }

        let func = args[0].clone();

        // Validate that the argument is callable
        match &func {
            PyValue::Function(_) | PyValue::Builtin(_) | PyValue::BoundMethod(_) => {
                // Return a classmethod descriptor wrapping the function
                Ok(PyValue::ClassMethod(Box::new(func)))
            }
            _ => {
                // Python allows any object to be wrapped in classmethod
                // even if it's not callable (error happens at call time)
                Ok(PyValue::ClassMethod(Box::new(func)))
            }
        }
    })
}

/// Create the input builtin
pub fn builtin_input() -> PyBuiltinFunction {
    PyBuiltinFunction::new("input", |args| {
        // Print prompt if provided
        if let Some(PyValue::Str(prompt)) = args.first() {
            print!("{}", prompt);
            use std::io::Write;
            std::io::stdout().flush().ok();
        }

        // Read line from stdin
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).map_err(|e| RuntimeError::OsError {
            message: e.to_string(),
        })?;

        // Remove trailing newline
        if input.ends_with('\n') {
            input.pop();
            if input.ends_with('\r') {
                input.pop();
            }
        }

        Ok(PyValue::Str(Arc::from(input)))
    })
}

// ===== I/O and Execution Functions (Task 7.5) =====

/// Create the open builtin
pub fn builtin_open() -> PyBuiltinFunction {
    PyBuiltinFunction::new("open", |args| {
        let path = match args.first() {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str or path-like", v.type_name())),
            None => return Err(RuntimeError::type_error("1-3 arguments", "0 arguments")),
        };

        let mode = match args.get(1) {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str", v.type_name())),
            None => "r".to_string(),
        };

        let encoding = match args.get(2) {
            Some(PyValue::Str(s)) => Some(s.to_string()),
            Some(PyValue::None) | None => None,
            Some(v) => return Err(RuntimeError::type_error("str or None", v.type_name())),
        };

        // Parse mode string
        let (read, write, append, binary, create, truncate) = parse_file_mode(&mode)?;

        // Build open options
        use std::fs::OpenOptions;
        let mut options = OpenOptions::new();
        options.read(read);
        options.write(write || append);
        options.append(append);
        options.create(create || write || append);
        options.truncate(truncate);

        // Open the file
        let file = options.open(&path).map_err(|e| RuntimeError::OsError {
            message: format!("Cannot open '{}': {}", path, e),
        })?;

        // Return a file handle representation
        // For now, we return a dict with file info since we don't have a proper file object
        use crate::pydict::PyKey;
        let file_dict = crate::PyDict::new();
        file_dict.setitem(PyKey::Str(Arc::from("path")), PyValue::Str(Arc::from(path)));
        file_dict.setitem(PyKey::Str(Arc::from("mode")), PyValue::Str(Arc::from(mode)));
        file_dict.setitem(PyKey::Str(Arc::from("binary")), PyValue::Bool(binary));
        file_dict.setitem(
            PyKey::Str(Arc::from("encoding")),
            match encoding {
                Some(enc) => PyValue::Str(Arc::from(enc)),
                None => PyValue::Str(Arc::from("utf-8")),
            },
        );
        file_dict.setitem(PyKey::Str(Arc::from("closed")), PyValue::Bool(false));

        // Store file handle (simplified - in real impl would store actual handle)
        drop(file); // For now, just validate the file can be opened

        Ok(PyValue::Dict(Arc::new(file_dict)))
    })
}

/// Parse file mode string into flags
fn parse_file_mode(mode: &str) -> RuntimeResult<(bool, bool, bool, bool, bool, bool)> {
    let mut read = false;
    let mut write = false;
    let mut append = false;
    let mut binary = false;
    let mut create = false;
    let mut truncate = false;

    for c in mode.chars() {
        match c {
            'r' => read = true,
            'w' => {
                write = true;
                create = true;
                truncate = true;
            }
            'a' => {
                append = true;
                create = true;
            }
            'x' => {
                write = true;
                create = true;
            }
            'b' => binary = true,
            't' => {} // text mode is default
            '+' => {
                read = true;
                write = true;
            }
            _ => return Err(RuntimeError::value_error(format!("invalid mode: '{}'", mode))),
        }
    }

    // Default to read if no mode specified
    if !read && !write && !append {
        read = true;
    }

    Ok((read, write, append, binary, create, truncate))
}

/// Create the exec builtin
pub fn builtin_exec() -> PyBuiltinFunction {
    PyBuiltinFunction::new("exec", |args| {
        let source = match args.first() {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str or code object", v.type_name())),
            None => return Err(RuntimeError::type_error("1-3 arguments", "0 arguments")),
        };

        // Note: Full exec implementation requires compiler integration
        // For now, return None and log that exec was called
        #[cfg(debug_assertions)]
        eprintln!("[exec] Would execute: {}", source.lines().next().unwrap_or(""));

        Ok(PyValue::None)
    })
}

/// Create the eval builtin
pub fn builtin_eval() -> PyBuiltinFunction {
    PyBuiltinFunction::new("eval", |args| {
        let source = match args.first() {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str or code object", v.type_name())),
            None => return Err(RuntimeError::type_error("1-3 arguments", "0 arguments")),
        };

        // Try to evaluate simple expressions
        let trimmed = source.trim();

        // Handle simple literals
        if let Ok(i) = trimmed.parse::<i64>() {
            return Ok(PyValue::Int(i));
        }
        if let Ok(f) = trimmed.parse::<f64>() {
            return Ok(PyValue::Float(f));
        }
        if trimmed == "True" {
            return Ok(PyValue::Bool(true));
        }
        if trimmed == "False" {
            return Ok(PyValue::Bool(false));
        }
        if trimmed == "None" {
            return Ok(PyValue::None);
        }
        // Handle string literals
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let inner = &trimmed[1..trimmed.len() - 1];
            return Ok(PyValue::Str(Arc::from(inner)));
        }

        // Note: Full eval implementation requires compiler integration
        Err(RuntimeError::internal_error(format!(
            "eval() cannot evaluate complex expression: {}",
            trimmed
        )))
    })
}

/// Create the compile builtin
pub fn builtin_compile() -> PyBuiltinFunction {
    PyBuiltinFunction::new("compile", |args| {
        let source = match args.first() {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str", v.type_name())),
            None => return Err(RuntimeError::type_error("3 arguments", "0 arguments")),
        };

        let filename = match args.get(1) {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str", v.type_name())),
            None => return Err(RuntimeError::type_error("3 arguments", "1 argument")),
        };

        let mode = match args.get(2) {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str", v.type_name())),
            None => return Err(RuntimeError::type_error("3 arguments", "2 arguments")),
        };

        // Validate mode
        if mode != "exec" && mode != "eval" && mode != "single" {
            return Err(RuntimeError::value_error(format!(
                "compile() mode must be 'exec', 'eval' or 'single', not '{}'",
                mode
            )));
        }

        // Note: Full compile implementation requires compiler integration
        // Return a code object representation
        use crate::pydict::PyKey;
        let code_dict = crate::PyDict::new();
        code_dict.setitem(PyKey::Str(Arc::from("source")), PyValue::Str(Arc::from(source)));
        code_dict.setitem(PyKey::Str(Arc::from("filename")), PyValue::Str(Arc::from(filename)));
        code_dict.setitem(PyKey::Str(Arc::from("mode")), PyValue::Str(Arc::from(mode)));
        code_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("code")));

        Ok(PyValue::Dict(Arc::new(code_dict)))
    })
}

/// Create an enhanced print builtin with file parameter support
pub fn builtin_print_enhanced() -> PyBuiltinFunction {
    PyBuiltinFunction::new("print", |args| {
        // Parse keyword-like arguments from the end
        // In a real implementation, we'd have proper kwargs support
        let sep = " ";
        let end = "\n";
        let file_output = None::<String>;
        let flush = false;

        // For now, just print all args with default sep/end
        let output: Vec<String> = args.iter().map(format_value).collect();

        match file_output {
            Some(path) => {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(&path)
                    .map_err(|e| RuntimeError::OsError {
                        message: format!("Cannot write to '{}': {}", path, e),
                    })?;
                write!(file, "{}{}", output.join(sep), end).map_err(|e| RuntimeError::OsError {
                    message: e.to_string(),
                })?;
                if flush {
                    file.flush().ok();
                }
            }
            None => {
                print!("{}{}", output.join(sep), end);
                if flush {
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
            }
        }

        Ok(PyValue::None)
    })
}

/// Create the format builtin
pub fn builtin_format() -> PyBuiltinFunction {
    PyBuiltinFunction::new("format", |args| {
        let value = match args.first() {
            Some(v) => v,
            None => return Err(RuntimeError::type_error("1-2 arguments", "0 arguments")),
        };

        let format_spec = match args.get(1) {
            Some(PyValue::Str(s)) => s.to_string(),
            Some(v) => return Err(RuntimeError::type_error("str", v.type_name())),
            None => String::new(),
        };

        // Handle basic format specs
        let result = if format_spec.is_empty() {
            format_value(value)
        } else {
            // Parse format spec: [[fill]align][sign][#][0][width][,][.precision][type]
            match value {
                PyValue::Int(i) => format_int(*i, &format_spec)?,
                PyValue::Float(f) => format_float(*f, &format_spec)?,
                PyValue::Str(s) => format_string(s, &format_spec)?,
                _ => format_value(value),
            }
        };

        Ok(PyValue::Str(Arc::from(result)))
    })
}

/// Format an integer with format spec
fn format_int(value: i64, spec: &str) -> RuntimeResult<String> {
    let last_char = spec.chars().last().unwrap_or('d');
    match last_char {
        'd' | 'n' => Ok(format!("{}", value)),
        'b' => Ok(format!("{:b}", value)),
        'o' => Ok(format!("{:o}", value)),
        'x' => Ok(format!("{:x}", value)),
        'X' => Ok(format!("{:X}", value)),
        'e' => Ok(format!("{:e}", value as f64)),
        'E' => Ok(format!("{:E}", value as f64)),
        'f' | 'F' => Ok(format!("{:.6}", value as f64)),
        '%' => Ok(format!("{:.6}%", value as f64 * 100.0)),
        _ => Ok(format!("{}", value)),
    }
}

/// Format a float with format spec
fn format_float(value: f64, spec: &str) -> RuntimeResult<String> {
    let last_char = spec.chars().last().unwrap_or('g');
    match last_char {
        'e' => Ok(format!("{:e}", value)),
        'E' => Ok(format!("{:E}", value)),
        'f' | 'F' => Ok(format!("{:.6}", value)),
        'g' | 'G' => Ok(format!("{}", value)),
        '%' => Ok(format!("{:.6}%", value * 100.0)),
        _ => Ok(format!("{}", value)),
    }
}

/// Format a string with format spec
fn format_string(value: &str, spec: &str) -> RuntimeResult<String> {
    // Parse width and alignment
    let width: usize = spec
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0);

    if width == 0 || value.len() >= width {
        return Ok(value.to_string());
    }

    let align = if spec.starts_with('<') {
        '<'
    } else if spec.starts_with('>') {
        '>'
    } else if spec.starts_with('^') {
        '^'
    } else {
        '<' // default left align for strings
    };

    let padding = width - value.len();
    match align {
        '<' => Ok(format!("{}{}", value, " ".repeat(padding))),
        '>' => Ok(format!("{}{}", " ".repeat(padding), value)),
        '^' => {
            let left = padding / 2;
            let right = padding - left;
            Ok(format!("{}{}{}", " ".repeat(left), value, " ".repeat(right)))
        }
        _ => Ok(value.to_string()),
    }
}

/// Create the ascii builtin
pub fn builtin_ascii() -> PyBuiltinFunction {
    PyBuiltinFunction::new("ascii", |args| match args.first() {
        Some(PyValue::Str(s)) => {
            let mut result = String::with_capacity(s.len() + 2);
            result.push('\'');
            for c in s.chars() {
                // Only push printable ASCII directly (excluding quotes and backslash)
                if c.is_ascii() && !c.is_ascii_control() && c != '\'' && c != '\\' {
                    result.push(c);
                } else {
                    match c {
                        '\'' => result.push_str("\\'"),
                        '\\' => result.push_str("\\\\"),
                        '\n' => result.push_str("\\n"),
                        '\r' => result.push_str("\\r"),
                        '\t' => result.push_str("\\t"),
                        _ => {
                            let code = c as u32;
                            if code < 0x100 {
                                result.push_str(&format!("\\x{:02x}", code));
                            } else if code < 0x10000 {
                                result.push_str(&format!("\\u{:04x}", code));
                            } else {
                                result.push_str(&format!("\\U{:08x}", code));
                            }
                        }
                    }
                }
            }
            result.push('\'');
            Ok(PyValue::Str(Arc::from(result)))
        }
        Some(v) => Ok(PyValue::Str(Arc::from(repr_value(v)))),
        None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
    })
}

/// Create the globals builtin
pub fn builtin_globals() -> PyBuiltinFunction {
    PyBuiltinFunction::new("globals", |_args| {
        // Return an empty dict - real implementation needs frame access
        Ok(PyValue::Dict(Arc::new(crate::PyDict::new())))
    })
}

/// Create the locals builtin
pub fn builtin_locals() -> PyBuiltinFunction {
    PyBuiltinFunction::new("locals", |_args| {
        // Return an empty dict - real implementation needs frame access
        Ok(PyValue::Dict(Arc::new(crate::PyDict::new())))
    })
}

/// Create the vars builtin
pub fn builtin_vars() -> PyBuiltinFunction {
    PyBuiltinFunction::new("vars", |args| match args.first() {
        Some(PyValue::Dict(d)) => Ok(PyValue::Dict(Arc::clone(d))),
        Some(_) => {
            // Would return __dict__ of object
            Ok(PyValue::Dict(Arc::new(crate::PyDict::new())))
        }
        None => {
            // Return locals() when called without args
            Ok(PyValue::Dict(Arc::new(crate::PyDict::new())))
        }
    })
}

/// Create the dir builtin
pub fn builtin_dir() -> PyBuiltinFunction {
    PyBuiltinFunction::new("dir", |args| match args.first() {
        Some(PyValue::Dict(d)) => {
            let keys: Vec<PyValue> = d
                .keys()
                .iter()
                .map(|k| match k {
                    crate::pydict::PyKey::Str(s) => PyValue::Str(Arc::clone(s)),
                    crate::pydict::PyKey::Int(i) => PyValue::Int(*i),
                    crate::pydict::PyKey::Bool(b) => PyValue::Bool(*b),
                    crate::pydict::PyKey::None => PyValue::None,
                    crate::pydict::PyKey::Tuple(t) => {
                        let values: Vec<PyValue> = t.iter().map(|k| k.to_value()).collect();
                        PyValue::Tuple(Arc::new(crate::PyTuple::from_values(values)))
                    }
                })
                .collect();
            Ok(PyValue::List(Arc::new(crate::PyList::from_values(keys))))
        }
        Some(_) => {
            // Would return attributes of object
            Ok(PyValue::List(Arc::new(crate::PyList::new())))
        }
        None => {
            // Return names in current scope
            Ok(PyValue::List(Arc::new(crate::PyList::new())))
        }
    })
}

/// Create the setattr builtin
/// setattr(object, name, value) -> None
/// Set a named attribute on an object; setattr(x, 'y', v) is equivalent to x.y = v.
pub fn builtin_setattr() -> PyBuiltinFunction {
    PyBuiltinFunction::new("setattr", |args| {
        if args.len() != 3 {
            return Err(RuntimeError::type_error(
                "3 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let obj = &args[0];
        let name = match &args[1] {
            PyValue::Str(s) => s.to_string(),
            _ => {
                return Err(RuntimeError::type_error("str", args[1].type_name()));
            }
        };
        let value = args[2].clone();

        match obj {
            PyValue::Instance(inst) => {
                inst.set_attr(name, value);
                Ok(PyValue::None)
            }
            PyValue::Type(ty) => {
                ty.set_attr(name, value);
                Ok(PyValue::None)
            }
            PyValue::Module(m) => {
                m.dict.insert(name, value);
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error("object with settable attributes", obj.type_name())),
        }
    })
}

/// Create the delattr builtin
/// delattr(object, name) -> None
/// Delete a named attribute on an object; delattr(x, 'y') is equivalent to del x.y.
pub fn builtin_delattr() -> PyBuiltinFunction {
    PyBuiltinFunction::new("delattr", |args| {
        if args.len() != 2 {
            return Err(RuntimeError::type_error(
                "2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        let obj = &args[0];
        let name = match &args[1] {
            PyValue::Str(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::type_error("str", args[1].type_name()));
            }
        };

        match obj {
            PyValue::Instance(inst) => {
                if inst.del_attr(name) {
                    Ok(PyValue::None)
                } else {
                    Err(RuntimeError::attribute_error(inst.class_name(), name))
                }
            }
            PyValue::Module(m) => {
                if m.dict.remove(name).is_some() {
                    Ok(PyValue::None)
                } else {
                    Err(RuntimeError::attribute_error(&*m.name, name))
                }
            }
            _ => Err(RuntimeError::type_error("object with deletable attributes", obj.type_name())),
        }
    })
}

/// Create the __build_class__ builtin
/// This is the internal function used to build class objects.
/// Called as: __build_class__(class_body_func, class_name, *bases, **kwargs)
pub fn builtin_build_class() -> PyBuiltinFunction {
    PyBuiltinFunction::new("__build_class__", |args| {
        // Minimum 2 arguments: class_body_func and class_name
        if args.len() < 2 {
            return Err(RuntimeError::type_error(
                "at least 2 arguments",
                format!("{} arguments", args.len()),
            ));
        }

        // First argument is the class body function
        let _class_body = &args[0];

        // Second argument is the class name
        let class_name = match &args[1] {
            PyValue::Str(s) => s.to_string(),
            _ => return Err(RuntimeError::type_error("str", args[1].type_name())),
        };

        // Remaining arguments are base classes
        let mut bases: Vec<Arc<crate::types::PyType>> = Vec::new();
        for arg in args.iter().skip(2) {
            match arg {
                PyValue::Type(t) => bases.push(Arc::clone(t)),
                _ => {
                    // For now, skip non-type bases (could be metaclass keyword args)
                }
            }
        }

        // Create the PyType with bases
        let class = if bases.is_empty() {
            crate::types::PyType::new(&class_name)
        } else {
            crate::types::PyType::with_bases(&class_name, bases)
        };

        // In a full implementation, we would:
        // 1. Create a new namespace dict
        // 2. Execute the class body function with the namespace as locals
        // 3. Extract methods and attributes from the namespace
        // For now, we create an empty class

        Ok(PyValue::Type(Arc::new(class)))
    })
}

/// Get all builtin functions
pub fn get_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        builtin_print(),
        builtin_len(),
        builtin_type(),
        builtin_int(),
        builtin_float(),
        builtin_str(),
        builtin_bool(),
        builtin_abs(),
        builtin_min(),
        builtin_max(),
        builtin_sum(),
        builtin_range(),
        // Type constructors (Task 7.1)
        builtin_list(),
        builtin_dict(),
        builtin_set(),
        builtin_tuple(),
        builtin_bytes(),
        builtin_object(),
        // Iteration functions (Task 7.2)
        builtin_enumerate(),
        builtin_zip(),
        builtin_map(),
        builtin_filter(),
        builtin_sorted(),
        builtin_reversed(),
        builtin_iter(),
        builtin_next(),
        builtin_all(),
        builtin_any(),
        // Introspection functions (Task 7.4)
        builtin_isinstance(),
        builtin_issubclass(),
        builtin_super(),
        builtin_hasattr(),
        builtin_getattr(),
        builtin_callable(),
        builtin_id(),
        builtin_hash(),
        builtin_repr(),
        builtin_ord(),
        builtin_chr(),
        builtin_hex(),
        builtin_bin(),
        builtin_oct(),
        builtin_pow(),
        builtin_round(),
        builtin_divmod(),
        builtin_input(),
        // I/O and execution functions (Task 7.5)
        builtin_open(),
        builtin_exec(),
        builtin_eval(),
        builtin_compile(),
        builtin_format(),
        builtin_ascii(),
        builtin_globals(),
        builtin_locals(),
        builtin_vars(),
        builtin_dir(),
        builtin_setattr(),
        builtin_delattr(),
        // Class building (Task 8.1)
        builtin_build_class(),
        // Descriptor builtins (Task 17.4)
        builtin_property(),
        builtin_staticmethod(),
        builtin_classmethod(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_len() {
        let len_fn = builtin_len();

        let result = len_fn.call(&[PyValue::Str(Arc::from("hello"))]).unwrap();
        assert!(matches!(result, PyValue::Int(5)));
    }

    #[test]
    fn test_builtin_int() {
        let int_fn = builtin_int();

        let result = int_fn.call(&[PyValue::Str(Arc::from("42"))]).unwrap();
        assert!(matches!(result, PyValue::Int(42)));

        let result = int_fn.call(&[PyValue::Float(3.125)]).unwrap();
        assert!(matches!(result, PyValue::Int(3)));
    }

    #[test]
    fn test_builtin_abs() {
        let abs_fn = builtin_abs();

        let result = abs_fn.call(&[PyValue::Int(-42)]).unwrap();
        assert!(matches!(result, PyValue::Int(42)));
    }

    #[test]
    fn test_builtin_range() {
        let range_fn = builtin_range();

        let result = range_fn.call(&[PyValue::Int(5)]).unwrap();
        if let PyValue::List(list) = result {
            assert_eq!(list.len(), 5);
        } else {
            panic!("Expected list");
        }
    }

    // ===== Task 7.5 Tests =====

    #[test]
    fn test_builtin_eval_literals() {
        let eval_fn = builtin_eval();

        // Test integer literal
        let result = eval_fn.call(&[PyValue::Str(Arc::from("42"))]).unwrap();
        assert!(matches!(result, PyValue::Int(42)));

        // Test float literal
        let result = eval_fn.call(&[PyValue::Str(Arc::from("3.14"))]).unwrap();
        if let PyValue::Float(f) = result {
            // Compare against the expected value (not std::f64::consts::PI)
            #[allow(clippy::approx_constant)]
            let expected = 3.14_f64;
            assert!((f - expected).abs() < 0.001);
        } else {
            panic!("Expected float");
        }

        // Test boolean literals
        let result = eval_fn.call(&[PyValue::Str(Arc::from("True"))]).unwrap();
        assert!(matches!(result, PyValue::Bool(true)));

        let result = eval_fn.call(&[PyValue::Str(Arc::from("False"))]).unwrap();
        assert!(matches!(result, PyValue::Bool(false)));

        // Test None
        let result = eval_fn.call(&[PyValue::Str(Arc::from("None"))]).unwrap();
        assert!(matches!(result, PyValue::None));

        // Test string literal
        let result = eval_fn.call(&[PyValue::Str(Arc::from("'hello'"))]).unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "hello");
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_builtin_compile() {
        let compile_fn = builtin_compile();

        let result = compile_fn
            .call(&[
                PyValue::Str(Arc::from("x = 1")),
                PyValue::Str(Arc::from("<string>")),
                PyValue::Str(Arc::from("exec")),
            ])
            .unwrap();

        if let PyValue::Dict(d) = result {
            use crate::pydict::PyKey;
            assert!(d.contains(&PyKey::Str(Arc::from("source"))));
            assert!(d.contains(&PyKey::Str(Arc::from("filename"))));
            assert!(d.contains(&PyKey::Str(Arc::from("mode"))));
        } else {
            panic!("Expected dict");
        }
    }

    #[test]
    fn test_builtin_compile_invalid_mode() {
        let compile_fn = builtin_compile();

        let result = compile_fn.call(&[
            PyValue::Str(Arc::from("x = 1")),
            PyValue::Str(Arc::from("<string>")),
            PyValue::Str(Arc::from("invalid")),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_format_int() {
        let format_fn = builtin_format();

        // Default format
        let result = format_fn.call(&[PyValue::Int(42)]).unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "42");
        } else {
            panic!("Expected string");
        }

        // Binary format
        let result = format_fn.call(&[PyValue::Int(42), PyValue::Str(Arc::from("b"))]).unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "101010");
        } else {
            panic!("Expected string");
        }

        // Hex format
        let result = format_fn.call(&[PyValue::Int(255), PyValue::Str(Arc::from("x"))]).unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "ff");
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_builtin_ascii() {
        let ascii_fn = builtin_ascii();

        // ASCII string
        let result = ascii_fn.call(&[PyValue::Str(Arc::from("hello"))]).unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "'hello'");
        } else {
            panic!("Expected string");
        }

        // String with newline - the input has actual newline char
        let input_with_newline = "hello\nworld";
        let result = ascii_fn.call(&[PyValue::Str(Arc::from(input_with_newline))]).unwrap();
        if let PyValue::Str(s) = result {
            // The output should have escaped newline
            assert!(s.contains("\\n"), "Expected escaped newline in: {}", s);
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_parse_file_mode() {
        // Read mode
        let (r, w, a, b, _c, _t) = parse_file_mode("r").unwrap();
        assert!(r && !w && !a && !b);

        // Write mode
        let (r, w, a, b, c, t) = parse_file_mode("w").unwrap();
        assert!(!r && w && !a && !b && c && t);

        // Append mode
        let (r, w, a, b, c, _t) = parse_file_mode("a").unwrap();
        assert!(!r && !w && a && !b && c);

        // Binary read mode
        let (r, w, a, b, _c, _t) = parse_file_mode("rb").unwrap();
        assert!(r && !w && !a && b);

        // Read-write mode
        let (r, w, a, b, _c, _t) = parse_file_mode("r+").unwrap();
        assert!(r && w && !a && !b);

        // Invalid mode
        let result = parse_file_mode("z");
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_globals_locals() {
        let globals_fn = builtin_globals();
        let locals_fn = builtin_locals();

        // Both should return empty dicts for now
        let result = globals_fn.call(&[]).unwrap();
        assert!(matches!(result, PyValue::Dict(_)));

        let result = locals_fn.call(&[]).unwrap();
        assert!(matches!(result, PyValue::Dict(_)));
    }

    #[test]
    fn test_builtin_dir() {
        let dir_fn = builtin_dir();

        // Dir on dict should return keys
        use crate::pydict::PyKey;
        let dict = crate::PyDict::new();
        dict.setitem(PyKey::Str(Arc::from("a")), PyValue::Int(1));
        dict.setitem(PyKey::Str(Arc::from("b")), PyValue::Int(2));

        let result = dir_fn.call(&[PyValue::Dict(Arc::new(dict))]).unwrap();
        if let PyValue::List(l) = result {
            assert_eq!(l.len(), 2);
        } else {
            panic!("Expected list");
        }
    }

    // ===== Task 17.4 Tests: property(), staticmethod(), classmethod() =====

    #[test]
    fn test_builtin_property() {
        let property_fn = builtin_property();

        // Create a property with just a getter
        let getter = PyValue::Function(Arc::new(crate::pyfunction::PyFunction::new(
            "getter".to_string(),
            crate::pyfunction::CodeRef {
                bytecode_offset: 0,
                num_locals: 1,
                stack_size: 1,
                num_args: 1,
                num_kwonly_args: 0,
            },
            vec![],
        )));

        let result = property_fn.call(&[getter]).unwrap();
        assert!(matches!(result, PyValue::Property(_)));
        assert_eq!(result.type_name(), "property");
    }

    #[test]
    fn test_builtin_property_with_doc() {
        let property_fn = builtin_property();

        // Create a property with getter, setter, deleter, and doc
        let getter = PyValue::Function(Arc::new(crate::pyfunction::PyFunction::new(
            "getter".to_string(),
            crate::pyfunction::CodeRef {
                bytecode_offset: 0,
                num_locals: 1,
                stack_size: 1,
                num_args: 1,
                num_kwonly_args: 0,
            },
            vec![],
        )));
        let setter = PyValue::Function(Arc::new(crate::pyfunction::PyFunction::new(
            "setter".to_string(),
            crate::pyfunction::CodeRef {
                bytecode_offset: 0,
                num_locals: 2,
                stack_size: 1,
                num_args: 2,
                num_kwonly_args: 0,
            },
            vec![],
        )));
        let doc = PyValue::Str(Arc::from("The x property"));

        let result = property_fn.call(&[getter, setter, PyValue::None, doc]).unwrap();
        if let PyValue::Property(p) = result {
            assert_eq!(p.get_doc(), Some("The x property"));
        } else {
            panic!("Expected Property");
        }
    }

    #[test]
    fn test_builtin_staticmethod() {
        let staticmethod_fn = builtin_staticmethod();

        // Create a staticmethod wrapping a function
        let func = PyValue::Function(Arc::new(crate::pyfunction::PyFunction::new(
            "my_static".to_string(),
            crate::pyfunction::CodeRef {
                bytecode_offset: 0,
                num_locals: 1,
                stack_size: 1,
                num_args: 1,
                num_kwonly_args: 0,
            },
            vec![],
        )));

        let result = staticmethod_fn.call(&[func]).unwrap();
        assert!(matches!(result, PyValue::StaticMethod(_)));
        assert_eq!(result.type_name(), "staticmethod");
    }

    #[test]
    fn test_builtin_staticmethod_no_args() {
        let staticmethod_fn = builtin_staticmethod();

        // staticmethod() with no arguments should error
        let result = staticmethod_fn.call(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_classmethod() {
        let classmethod_fn = builtin_classmethod();

        // Create a classmethod wrapping a function
        let func = PyValue::Function(Arc::new(crate::pyfunction::PyFunction::new(
            "my_classmethod".to_string(),
            crate::pyfunction::CodeRef {
                bytecode_offset: 0,
                num_locals: 2,
                stack_size: 1,
                num_args: 2,
                num_kwonly_args: 0,
            },
            vec![],
        )));

        let result = classmethod_fn.call(&[func]).unwrap();
        assert!(matches!(result, PyValue::ClassMethod(_)));
        assert_eq!(result.type_name(), "classmethod");
    }

    #[test]
    fn test_builtin_classmethod_no_args() {
        let classmethod_fn = builtin_classmethod();

        // classmethod() with no arguments should error
        let result = classmethod_fn.call(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_staticmethod_wraps_any_value() {
        let staticmethod_fn = builtin_staticmethod();

        // Python allows wrapping any value in staticmethod
        let result = staticmethod_fn.call(&[PyValue::Int(42)]).unwrap();
        if let PyValue::StaticMethod(inner) = result {
            assert!(matches!(*inner, PyValue::Int(42)));
        } else {
            panic!("Expected StaticMethod");
        }
    }

    #[test]
    fn test_classmethod_wraps_any_value() {
        let classmethod_fn = builtin_classmethod();

        // Python allows wrapping any value in classmethod
        let result = classmethod_fn.call(&[PyValue::Str(Arc::from("test"))]).unwrap();
        if let PyValue::ClassMethod(inner) = result {
            if let PyValue::Str(s) = *inner {
                assert_eq!(s.as_ref(), "test");
            } else {
                panic!("Expected Str inside ClassMethod");
            }
        } else {
            panic!("Expected ClassMethod");
        }
    }

    // ===== Task 17.5 Tests: enumerate(), zip(), map(), filter() =====

    #[test]
    fn test_builtin_enumerate() {
        let enumerate_fn = builtin_enumerate();

        // Test enumerate on a list
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Str(Arc::from("a")),
            PyValue::Str(Arc::from("b")),
            PyValue::Str(Arc::from("c")),
        ])));

        let result = enumerate_fn.call(&[list]).unwrap();
        if let PyValue::List(l) = result {
            assert_eq!(l.len(), 3);
            // Check first element is (0, "a")
            if let PyValue::Tuple(t) = &l.to_vec()[0] {
                let items = t.to_vec();
                assert!(matches!(items[0], PyValue::Int(0)));
                if let PyValue::Str(s) = &items[1] {
                    assert_eq!(s.as_ref(), "a");
                }
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_enumerate_with_start() {
        let enumerate_fn = builtin_enumerate();

        // Test enumerate with start parameter
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Str(Arc::from("x")),
            PyValue::Str(Arc::from("y")),
        ])));

        let result = enumerate_fn.call(&[list, PyValue::Int(10)]).unwrap();
        if let PyValue::List(l) = result {
            assert_eq!(l.len(), 2);
            // Check first element is (10, "x")
            if let PyValue::Tuple(t) = &l.to_vec()[0] {
                let items = t.to_vec();
                assert!(matches!(items[0], PyValue::Int(10)));
            }
            // Check second element is (11, "y")
            if let PyValue::Tuple(t) = &l.to_vec()[1] {
                let items = t.to_vec();
                assert!(matches!(items[0], PyValue::Int(11)));
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_zip() {
        let zip_fn = builtin_zip();

        // Test zip on two lists
        let list1 = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Int(3),
        ])));
        let list2 = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Str(Arc::from("a")),
            PyValue::Str(Arc::from("b")),
            PyValue::Str(Arc::from("c")),
        ])));

        let result = zip_fn.call(&[list1, list2]).unwrap();
        if let PyValue::List(l) = result {
            assert_eq!(l.len(), 3);
            // Check first element is (1, "a")
            if let PyValue::Tuple(t) = &l.to_vec()[0] {
                let items = t.to_vec();
                assert!(matches!(items[0], PyValue::Int(1)));
                if let PyValue::Str(s) = &items[1] {
                    assert_eq!(s.as_ref(), "a");
                }
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_zip_unequal_lengths() {
        let zip_fn = builtin_zip();

        // Test zip with unequal length lists (should stop at shortest)
        let list1 = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(1),
            PyValue::Int(2),
        ])));
        let list2 = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Str(Arc::from("a")),
            PyValue::Str(Arc::from("b")),
            PyValue::Str(Arc::from("c")),
            PyValue::Str(Arc::from("d")),
        ])));

        let result = zip_fn.call(&[list1, list2]).unwrap();
        if let PyValue::List(l) = result {
            assert_eq!(l.len(), 2); // Should be 2, not 4
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_zip_empty() {
        let zip_fn = builtin_zip();

        // Test zip with no arguments
        let result = zip_fn.call(&[]).unwrap();
        if let PyValue::List(l) = result {
            assert_eq!(l.len(), 0);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_map_with_builtin() {
        let map_fn = builtin_map();

        // Test map with a builtin function (str)
        let str_fn = PyValue::Builtin(Arc::new(builtin_str()));
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Int(3),
        ])));

        let result = map_fn.call(&[str_fn, list]).unwrap();
        if let PyValue::List(l) = result {
            assert_eq!(l.len(), 3);
            // Check that integers were converted to strings
            if let PyValue::Str(s) = &l.to_vec()[0] {
                assert_eq!(s.as_ref(), "1");
            }
            if let PyValue::Str(s) = &l.to_vec()[1] {
                assert_eq!(s.as_ref(), "2");
            }
            if let PyValue::Str(s) = &l.to_vec()[2] {
                assert_eq!(s.as_ref(), "3");
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_map_not_enough_args() {
        let map_fn = builtin_map();

        // Test map with not enough arguments
        let result = map_fn.call(&[PyValue::None]);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_filter_with_none() {
        let filter_fn = builtin_filter();

        // Test filter with None (filter by truthiness)
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(0),
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Bool(false),
            PyValue::Bool(true),
            PyValue::Str(Arc::from("")),
            PyValue::Str(Arc::from("hello")),
        ])));

        let result = filter_fn.call(&[PyValue::None, list]).unwrap();
        if let PyValue::List(l) = result {
            // Should filter out 0, false, and ""
            assert_eq!(l.len(), 4);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_filter_with_builtin() {
        let filter_fn = builtin_filter();

        // Test filter with a builtin function (bool)
        let bool_fn = PyValue::Builtin(Arc::new(builtin_bool()));
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(0),
            PyValue::Int(1),
            PyValue::Int(2),
        ])));

        let result = filter_fn.call(&[bool_fn, list]).unwrap();
        if let PyValue::List(l) = result {
            // Should filter out 0
            assert_eq!(l.len(), 2);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_filter_wrong_args() {
        let filter_fn = builtin_filter();

        // Test filter with wrong number of arguments
        let result = filter_fn.call(&[PyValue::None]);
        assert!(result.is_err());
    }

    // ===== Task 17.6 Tests: sorted(), reversed(), min(), max(), sum() =====

    #[test]
    fn test_builtin_sorted() {
        let sorted_fn = builtin_sorted();

        // Test sorted on a list of integers
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(3),
            PyValue::Int(1),
            PyValue::Int(4),
            PyValue::Int(1),
            PyValue::Int(5),
        ])));

        let result = sorted_fn.call(&[list]).unwrap();
        if let PyValue::List(l) = result {
            let items = l.to_vec();
            assert_eq!(items.len(), 5);
            assert!(matches!(items[0], PyValue::Int(1)));
            assert!(matches!(items[1], PyValue::Int(1)));
            assert!(matches!(items[2], PyValue::Int(3)));
            assert!(matches!(items[3], PyValue::Int(4)));
            assert!(matches!(items[4], PyValue::Int(5)));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_sorted_strings() {
        let sorted_fn = builtin_sorted();

        // Test sorted on a list of strings
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Str(Arc::from("banana")),
            PyValue::Str(Arc::from("apple")),
            PyValue::Str(Arc::from("cherry")),
        ])));

        let result = sorted_fn.call(&[list]).unwrap();
        if let PyValue::List(l) = result {
            let items = l.to_vec();
            assert_eq!(items.len(), 3);
            if let PyValue::Str(s) = &items[0] {
                assert_eq!(s.as_ref(), "apple");
            }
            if let PyValue::Str(s) = &items[1] {
                assert_eq!(s.as_ref(), "banana");
            }
            if let PyValue::Str(s) = &items[2] {
                assert_eq!(s.as_ref(), "cherry");
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_reversed() {
        let reversed_fn = builtin_reversed();

        // Test reversed on a list
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Int(3),
        ])));

        let result = reversed_fn.call(&[list]).unwrap();
        if let PyValue::List(l) = result {
            let items = l.to_vec();
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], PyValue::Int(3)));
            assert!(matches!(items[1], PyValue::Int(2)));
            assert!(matches!(items[2], PyValue::Int(1)));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_reversed_string() {
        let reversed_fn = builtin_reversed();

        // Test reversed on a string
        let s = PyValue::Str(Arc::from("hello"));

        let result = reversed_fn.call(&[s]).unwrap();
        if let PyValue::List(l) = result {
            let items = l.to_vec();
            assert_eq!(items.len(), 5);
            if let PyValue::Str(c) = &items[0] {
                assert_eq!(c.as_ref(), "o");
            }
            if let PyValue::Str(c) = &items[4] {
                assert_eq!(c.as_ref(), "h");
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_builtin_min() {
        let min_fn = builtin_min();

        // Test min with multiple arguments
        let result = min_fn
            .call(&[
                PyValue::Int(5),
                PyValue::Int(2),
                PyValue::Int(8),
                PyValue::Int(1),
            ])
            .unwrap();
        assert!(matches!(result, PyValue::Int(1)));
    }

    #[test]
    fn test_builtin_min_strings() {
        let min_fn = builtin_min();

        // Test min with strings
        let result = min_fn
            .call(&[
                PyValue::Str(Arc::from("banana")),
                PyValue::Str(Arc::from("apple")),
                PyValue::Str(Arc::from("cherry")),
            ])
            .unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "apple");
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_builtin_min_no_args() {
        let min_fn = builtin_min();

        // Test min with no arguments should error
        let result = min_fn.call(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_max() {
        let max_fn = builtin_max();

        // Test max with multiple arguments
        let result = max_fn
            .call(&[
                PyValue::Int(5),
                PyValue::Int(2),
                PyValue::Int(8),
                PyValue::Int(1),
            ])
            .unwrap();
        assert!(matches!(result, PyValue::Int(8)));
    }

    #[test]
    fn test_builtin_max_strings() {
        let max_fn = builtin_max();

        // Test max with strings
        let result = max_fn
            .call(&[
                PyValue::Str(Arc::from("banana")),
                PyValue::Str(Arc::from("apple")),
                PyValue::Str(Arc::from("cherry")),
            ])
            .unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "cherry");
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_builtin_max_no_args() {
        let max_fn = builtin_max();

        // Test max with no arguments should error
        let result = max_fn.call(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_sum() {
        let sum_fn = builtin_sum();

        // Test sum on a list of integers
        let list = PyValue::List(Arc::new(crate::PyList::from_values(vec![
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Int(3),
            PyValue::Int(4),
            PyValue::Int(5),
        ])));

        let result = sum_fn.call(&[list]).unwrap();
        assert!(matches!(result, PyValue::Int(15)));
    }

    #[test]
    fn test_builtin_sum_empty() {
        let sum_fn = builtin_sum();

        // Test sum on an empty list
        let list = PyValue::List(Arc::new(crate::PyList::new()));

        let result = sum_fn.call(&[list]).unwrap();
        assert!(matches!(result, PyValue::Int(0)));
    }

    #[test]
    fn test_builtin_sum_non_iterable() {
        let sum_fn = builtin_sum();

        // Test sum on a non-iterable should error
        let result = sum_fn.call(&[PyValue::Int(42)]);
        assert!(result.is_err());
    }

    // ===== Task 17.7 Tests: open() for file I/O =====

    #[test]
    fn test_builtin_open_read_mode() {
        let open_fn = builtin_open();

        // Create a temporary file for testing
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_py_test_open.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "Hello, World!").unwrap();
        }

        // Test opening the file in read mode
        let result = open_fn.call(&[PyValue::Str(Arc::from(test_file.to_str().unwrap()))]);

        // Clean up
        std::fs::remove_file(&test_file).ok();

        let result = result.unwrap();
        if let PyValue::Dict(d) = result {
            use crate::pydict::PyKey;
            // Check that the dict has the expected keys
            assert!(d.contains(&PyKey::Str(Arc::from("path"))));
            assert!(d.contains(&PyKey::Str(Arc::from("mode"))));
            assert!(d.contains(&PyKey::Str(Arc::from("closed"))));

            // Check mode is "r"
            if let Ok(PyValue::Str(mode)) = d.getitem(&PyKey::Str(Arc::from("mode"))) {
                assert_eq!(mode.as_ref(), "r");
            }
        } else {
            panic!("Expected dict");
        }
    }

    #[test]
    fn test_builtin_open_write_mode() {
        let open_fn = builtin_open();

        // Test opening a file in write mode
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_py_test_open_write.txt");

        let result = open_fn.call(&[
            PyValue::Str(Arc::from(test_file.to_str().unwrap())),
            PyValue::Str(Arc::from("w")),
        ]);

        // Clean up
        std::fs::remove_file(&test_file).ok();

        let result = result.unwrap();
        if let PyValue::Dict(d) = result {
            use crate::pydict::PyKey;
            // Check mode is "w"
            if let Ok(PyValue::Str(mode)) = d.getitem(&PyKey::Str(Arc::from("mode"))) {
                assert_eq!(mode.as_ref(), "w");
            }
        } else {
            panic!("Expected dict");
        }
    }

    #[test]
    fn test_builtin_open_nonexistent_file() {
        let open_fn = builtin_open();

        // Test opening a nonexistent file in read mode should error
        let result = open_fn.call(&[PyValue::Str(Arc::from("/nonexistent/path/to/file.txt"))]);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_open_invalid_mode() {
        let open_fn = builtin_open();

        // Create a temporary file for testing
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_py_test_open_invalid.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "test").unwrap();
        }

        // Test opening with invalid mode
        let result = open_fn.call(&[
            PyValue::Str(Arc::from(test_file.to_str().unwrap())),
            PyValue::Str(Arc::from("z")),
        ]);

        // Clean up
        std::fs::remove_file(&test_file).ok();

        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_open_binary_mode() {
        let open_fn = builtin_open();

        // Create a temporary file for testing
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_py_test_open_binary.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "binary test").unwrap();
        }

        // Test opening in binary read mode
        let result = open_fn.call(&[
            PyValue::Str(Arc::from(test_file.to_str().unwrap())),
            PyValue::Str(Arc::from("rb")),
        ]);

        // Clean up
        std::fs::remove_file(&test_file).ok();

        let result = result.unwrap();
        if let PyValue::Dict(d) = result {
            use crate::pydict::PyKey;
            // Check binary flag is true
            if let Ok(PyValue::Bool(binary)) = d.getitem(&PyKey::Str(Arc::from("binary"))) {
                assert!(binary);
            }
        } else {
            panic!("Expected dict");
        }
    }

    #[test]
    fn test_builtin_open_no_args() {
        let open_fn = builtin_open();

        // Test open with no arguments should error
        let result = open_fn.call(&[]);
        assert!(result.is_err());
    }
}
