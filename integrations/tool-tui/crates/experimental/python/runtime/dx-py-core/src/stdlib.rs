//! Standard library compatibility modules

#![allow(clippy::cloned_ref_to_slice_refs)]

use crate::pydict::PyKey;
use crate::pyfunction::PyBuiltinFunction;
use crate::pylist::PyValue;
use crate::{PyDict, PyList, PyTuple, RuntimeError, RuntimeResult};
use std::sync::Arc;

// ===== sys module (Task 7.6) =====

/// Get the sys module as a dict
pub fn sys_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Version info
    dict.setitem(
        PyKey::Str(Arc::from("version")),
        PyValue::Str(Arc::from("3.12.0 (dx-py 0.1.0)")),
    );
    dict.setitem(
        PyKey::Str(Arc::from("version_info")),
        PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
            PyValue::Int(3),
            PyValue::Int(12),
            PyValue::Int(0),
            PyValue::Str(Arc::from("final")),
            PyValue::Int(0),
        ]))),
    );

    // Platform info
    dict.setitem(PyKey::Str(Arc::from("platform")), PyValue::Str(Arc::from(get_platform())));
    dict.setitem(
        PyKey::Str(Arc::from("executable")),
        PyValue::Str(Arc::from(
            std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        )),
    );

    // Path and argv (will be populated at runtime)
    dict.setitem(
        PyKey::Str(Arc::from("path")),
        PyValue::List(Arc::new(PyList::from_values(vec![PyValue::Str(Arc::from("."))]))),
    );
    dict.setitem(PyKey::Str(Arc::from("argv")), PyValue::List(Arc::new(PyList::new())));

    // Modules dict (will be populated at runtime)
    dict.setitem(PyKey::Str(Arc::from("modules")), PyValue::Dict(Arc::new(PyDict::new())));

    // Recursion limit
    dict.setitem(PyKey::Str(Arc::from("_recursion_limit")), PyValue::Int(1000));

    // Standard streams (simplified - just markers for now)
    dict.setitem(PyKey::Str(Arc::from("stdin")), PyValue::Str(Arc::from("<stdin>")));
    dict.setitem(PyKey::Str(Arc::from("stdout")), PyValue::Str(Arc::from("<stdout>")));
    dict.setitem(PyKey::Str(Arc::from("stderr")), PyValue::Str(Arc::from("<stderr>")));

    // Implementation info
    dict.setitem(
        PyKey::Str(Arc::from("implementation")),
        PyValue::Dict(Arc::new({
            let impl_dict = PyDict::new();
            impl_dict.setitem(PyKey::Str(Arc::from("name")), PyValue::Str(Arc::from("dx-py")));
            impl_dict.setitem(
                PyKey::Str(Arc::from("version")),
                PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                    PyValue::Int(0),
                    PyValue::Int(1),
                    PyValue::Int(0),
                    PyValue::Str(Arc::from("alpha")),
                    PyValue::Int(0),
                ]))),
            );
            impl_dict
        })),
    );

    // Byte order
    dict.setitem(
        PyKey::Str(Arc::from("byteorder")),
        PyValue::Str(Arc::from(if cfg!(target_endian = "little") {
            "little"
        } else {
            "big"
        })),
    );

    // Max sizes
    dict.setitem(PyKey::Str(Arc::from("maxsize")), PyValue::Int(i64::MAX));
    dict.setitem(PyKey::Str(Arc::from("maxunicode")), PyValue::Int(0x10FFFF));

    // Float info (simplified)
    dict.setitem(
        PyKey::Str(Arc::from("float_info")),
        PyValue::Dict(Arc::new({
            let float_dict = PyDict::new();
            float_dict.setitem(PyKey::Str(Arc::from("max")), PyValue::Float(f64::MAX));
            float_dict.setitem(PyKey::Str(Arc::from("min")), PyValue::Float(f64::MIN_POSITIVE));
            float_dict.setitem(PyKey::Str(Arc::from("epsilon")), PyValue::Float(f64::EPSILON));
            float_dict
        })),
    );

    // Int info (simplified)
    dict.setitem(
        PyKey::Str(Arc::from("int_info")),
        PyValue::Dict(Arc::new({
            let int_dict = PyDict::new();
            int_dict.setitem(PyKey::Str(Arc::from("bits_per_digit")), PyValue::Int(30));
            int_dict.setitem(PyKey::Str(Arc::from("sizeof_digit")), PyValue::Int(4));
            int_dict
        })),
    );

    Arc::new(dict)
}

/// Get platform string
fn get_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

/// Create sys module builtins
pub fn sys_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        PyBuiltinFunction::new("exit", |args| {
            let code = match args.first() {
                Some(PyValue::Int(i)) => *i as i32,
                Some(PyValue::None) | None => 0,
                Some(PyValue::Str(s)) => {
                    eprintln!("{}", s);
                    1
                }
                _ => 1,
            };
            Err(RuntimeError::internal_error(format!("SystemExit: {}", code)))
        }),
        PyBuiltinFunction::new("exc_info", |_args| {
            // Return (None, None, None) when no exception is active
            Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                PyValue::None,
                PyValue::None,
                PyValue::None,
            ]))))
        }),
        PyBuiltinFunction::new("getrecursionlimit", |_args| Ok(PyValue::Int(1000))),
        PyBuiltinFunction::new("setrecursionlimit", |args| {
            match args.first() {
                Some(PyValue::Int(limit)) => {
                    if *limit < 1 {
                        return Err(RuntimeError::value_error("recursion limit must be positive"));
                    }
                    // In a real implementation, this would update a global
                    Ok(PyValue::None)
                }
                _ => Err(RuntimeError::type_error(
                    "int",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        PyBuiltinFunction::new("getsizeof", |args| {
            // Return approximate size in bytes
            match args.first() {
                Some(PyValue::Int(_)) => Ok(PyValue::Int(28)),
                Some(PyValue::Float(_)) => Ok(PyValue::Int(24)),
                Some(PyValue::Bool(_)) => Ok(PyValue::Int(28)),
                Some(PyValue::Str(s)) => Ok(PyValue::Int(49 + s.len() as i64)),
                Some(PyValue::List(l)) => Ok(PyValue::Int(56 + 8 * l.len() as i64)),
                Some(PyValue::Dict(d)) => Ok(PyValue::Int(232 + 24 * d.len() as i64)),
                Some(PyValue::Tuple(t)) => Ok(PyValue::Int(40 + 8 * t.len() as i64)),
                Some(PyValue::None) => Ok(PyValue::Int(16)),
                _ => Ok(PyValue::Int(0)),
            }
        }),
        PyBuiltinFunction::new("getdefaultencoding", |_args| Ok(PyValue::Str(Arc::from("utf-8")))),
        PyBuiltinFunction::new("getfilesystemencoding", |_args| {
            Ok(PyValue::Str(Arc::from("utf-8")))
        }),
        PyBuiltinFunction::new("intern", |args| {
            // Return the string as-is (interning is an optimization)
            match args.first() {
                Some(PyValue::Str(s)) => Ok(PyValue::Str(Arc::clone(s))),
                _ => Err(RuntimeError::type_error(
                    "str",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
    ]
}

// ===== os module (Task 7.7) =====

/// Get the os module as a dict
pub fn os_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // OS name
    dict.setitem(
        PyKey::Str(Arc::from("name")),
        PyValue::Str(Arc::from(if cfg!(target_os = "windows") {
            "nt"
        } else {
            "posix"
        })),
    );

    // Path separator
    dict.setitem(
        PyKey::Str(Arc::from("sep")),
        PyValue::Str(Arc::from(std::path::MAIN_SEPARATOR.to_string())),
    );
    dict.setitem(
        PyKey::Str(Arc::from("altsep")),
        if cfg!(target_os = "windows") {
            PyValue::Str(Arc::from("/"))
        } else {
            PyValue::None
        },
    );
    dict.setitem(PyKey::Str(Arc::from("extsep")), PyValue::Str(Arc::from(".")));
    dict.setitem(
        PyKey::Str(Arc::from("pathsep")),
        PyValue::Str(Arc::from(if cfg!(target_os = "windows") {
            ";"
        } else {
            ":"
        })),
    );
    dict.setitem(
        PyKey::Str(Arc::from("linesep")),
        PyValue::Str(Arc::from(if cfg!(target_os = "windows") {
            "\r\n"
        } else {
            "\n"
        })),
    );
    dict.setitem(
        PyKey::Str(Arc::from("devnull")),
        PyValue::Str(Arc::from(if cfg!(target_os = "windows") {
            "nul"
        } else {
            "/dev/null"
        })),
    );

    // Current directory
    dict.setitem(PyKey::Str(Arc::from("curdir")), PyValue::Str(Arc::from(".")));
    dict.setitem(PyKey::Str(Arc::from("pardir")), PyValue::Str(Arc::from("..")));

    // Environment variables
    let environ = PyDict::new();
    for (key, value) in std::env::vars() {
        environ.setitem(PyKey::Str(Arc::from(key)), PyValue::Str(Arc::from(value)));
    }
    dict.setitem(PyKey::Str(Arc::from("environ")), PyValue::Dict(Arc::new(environ)));

    Arc::new(dict)
}

/// Create os module builtins
pub fn os_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        PyBuiltinFunction::new("getcwd", |_args| {
            std::env::current_dir()
                .map(|p| PyValue::Str(Arc::from(p.to_string_lossy().to_string())))
                .map_err(|e| RuntimeError::OsError {
                    message: e.to_string(),
                })
        }),
        PyBuiltinFunction::new("chdir", |args| match args.first() {
            Some(PyValue::Str(path)) => std::env::set_current_dir(path.as_ref())
                .map(|_| PyValue::None)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot change directory to '{}': {}", path, e),
                }),
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("listdir", |args| {
            let path = match args.first() {
                Some(PyValue::Str(p)) => p.to_string(),
                None => ".".to_string(),
                _ => return Err(RuntimeError::type_error("str", args[0].type_name())),
            };

            let entries: RuntimeResult<Vec<PyValue>> = std::fs::read_dir(&path)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot list directory '{}': {}", path, e),
                })?
                .map(|entry| {
                    entry
                        .map(|e| {
                            PyValue::Str(Arc::from(e.file_name().to_string_lossy().to_string()))
                        })
                        .map_err(|e| RuntimeError::OsError {
                            message: e.to_string(),
                        })
                })
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(entries?))))
        }),
        PyBuiltinFunction::new("mkdir", |args| {
            let path = match args.first() {
                Some(PyValue::Str(p)) => p.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::fs::create_dir(&path)
                .map(|_| PyValue::None)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot create directory '{}': {}", path, e),
                })
        }),
        PyBuiltinFunction::new("makedirs", |args| {
            let path = match args.first() {
                Some(PyValue::Str(p)) => p.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let exist_ok = match args.get(1) {
                Some(PyValue::Bool(b)) => *b,
                _ => false,
            };

            if exist_ok && std::path::Path::new(&path).exists() {
                return Ok(PyValue::None);
            }

            std::fs::create_dir_all(&path).map(|_| PyValue::None).map_err(|e| {
                RuntimeError::OsError {
                    message: format!("Cannot create directories '{}': {}", path, e),
                }
            })
        }),
        PyBuiltinFunction::new("remove", |args| {
            let path = match args.first() {
                Some(PyValue::Str(p)) => p.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::fs::remove_file(&path)
                .map(|_| PyValue::None)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot remove file '{}': {}", path, e),
                })
        }),
        PyBuiltinFunction::new("rmdir", |args| {
            let path = match args.first() {
                Some(PyValue::Str(p)) => p.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::fs::remove_dir(&path)
                .map(|_| PyValue::None)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot remove directory '{}': {}", path, e),
                })
        }),
        PyBuiltinFunction::new("rename", |args| {
            let src = match args.first() {
                Some(PyValue::Str(p)) => p.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let dst = match args.get(1) {
                Some(PyValue::Str(p)) => p.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::fs::rename(&src, &dst)
                .map(|_| PyValue::None)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot rename '{}' to '{}': {}", src, dst, e),
                })
        }),
        PyBuiltinFunction::new("stat", |args| {
            let path = match args.first() {
                Some(PyValue::Str(p)) => p.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let metadata = std::fs::metadata(&path).map_err(|e| RuntimeError::OsError {
                message: format!("Cannot stat '{}': {}", path, e),
            })?;

            let stat_dict = PyDict::new();
            stat_dict
                .setitem(PyKey::Str(Arc::from("st_size")), PyValue::Int(metadata.len() as i64));
            stat_dict.setitem(
                PyKey::Str(Arc::from("st_mode")),
                PyValue::Int(if metadata.is_dir() { 0o40755 } else { 0o100644 }),
            );
            stat_dict.setitem(PyKey::Str(Arc::from("st_nlink")), PyValue::Int(1));
            stat_dict.setitem(PyKey::Str(Arc::from("st_uid")), PyValue::Int(0));
            stat_dict.setitem(PyKey::Str(Arc::from("st_gid")), PyValue::Int(0));

            // Time fields (simplified)
            let mtime = metadata
                .modified()
                .map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64
                })
                .unwrap_or(0);
            stat_dict.setitem(PyKey::Str(Arc::from("st_mtime")), PyValue::Int(mtime));
            stat_dict.setitem(PyKey::Str(Arc::from("st_atime")), PyValue::Int(mtime));
            stat_dict.setitem(PyKey::Str(Arc::from("st_ctime")), PyValue::Int(mtime));

            Ok(PyValue::Dict(Arc::new(stat_dict)))
        }),
        PyBuiltinFunction::new("getenv", |args| {
            match args.first() {
                Some(PyValue::Str(name)) => {
                    match std::env::var(name.as_ref()) {
                        Ok(val) => Ok(PyValue::Str(Arc::from(val))),
                        Err(_) => {
                            // Return default if provided
                            match args.get(1) {
                                Some(default) => Ok(default.clone()),
                                None => Ok(PyValue::None),
                            }
                        }
                    }
                }
                _ => Err(RuntimeError::type_error(
                    "str",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        PyBuiltinFunction::new("putenv", |args| {
            let name = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let value = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::env::set_var(&name, &value);
            Ok(PyValue::None)
        }),
        PyBuiltinFunction::new("unsetenv", |args| {
            let name = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::env::remove_var(&name);
            Ok(PyValue::None)
        }),
        PyBuiltinFunction::new("getpid", |_args| Ok(PyValue::Int(std::process::id() as i64))),
        PyBuiltinFunction::new("cpu_count", |_args| {
            Ok(PyValue::Int(
                std::thread::available_parallelism().map(|n| n.get() as i64).unwrap_or(1),
            ))
        }),
        PyBuiltinFunction::new("urandom", |args| {
            let n = match args.first() {
                Some(PyValue::Int(n)) if *n >= 0 => *n as usize,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("negative argument not allowed"))
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            // Generate random bytes
            use std::collections::hash_map::RandomState;
            use std::hash::{BuildHasher, Hasher};

            let mut bytes = Vec::with_capacity(n);
            let state = RandomState::new();
            for i in 0..n {
                let mut hasher = state.build_hasher();
                hasher.write_usize(i);
                bytes.push(PyValue::Int((hasher.finish() % 256) as i64));
            }

            Ok(PyValue::List(Arc::new(PyList::from_values(bytes))))
        }),
    ]
}

/// Get the os.path module as a dict
pub fn os_path_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Path separator
    dict.setitem(
        PyKey::Str(Arc::from("sep")),
        PyValue::Str(Arc::from(std::path::MAIN_SEPARATOR.to_string())),
    );

    Arc::new(dict)
}

/// Create os.path module builtins
pub fn os_path_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        PyBuiltinFunction::new("join", |args| {
            if args.is_empty() {
                return Ok(PyValue::Str(Arc::from("")));
            }

            let mut path = std::path::PathBuf::new();
            for arg in args {
                match arg {
                    PyValue::Str(s) => {
                        if s.starts_with(std::path::MAIN_SEPARATOR)
                            || (cfg!(target_os = "windows")
                                && s.len() >= 2
                                && s.chars().nth(1) == Some(':'))
                        {
                            // Absolute path - start fresh
                            path = std::path::PathBuf::from(s.as_ref());
                        } else {
                            path.push(s.as_ref());
                        }
                    }
                    _ => return Err(RuntimeError::type_error("str", arg.type_name())),
                }
            }

            Ok(PyValue::Str(Arc::from(path.to_string_lossy().to_string())))
        }),
        PyBuiltinFunction::new("exists", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                Ok(PyValue::Bool(std::path::Path::new(path.as_ref()).exists()))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("isfile", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                Ok(PyValue::Bool(std::path::Path::new(path.as_ref()).is_file()))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("isdir", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                Ok(PyValue::Bool(std::path::Path::new(path.as_ref()).is_dir()))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("isabs", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                Ok(PyValue::Bool(std::path::Path::new(path.as_ref()).is_absolute()))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("dirname", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                let p = std::path::Path::new(path.as_ref());
                Ok(PyValue::Str(Arc::from(
                    p.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                )))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("basename", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                let p = std::path::Path::new(path.as_ref());
                Ok(PyValue::Str(Arc::from(
                    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                )))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("split", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                let p = std::path::Path::new(path.as_ref());
                let dirname =
                    p.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let basename =
                    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

                Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                    PyValue::Str(Arc::from(dirname)),
                    PyValue::Str(Arc::from(basename)),
                ]))))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("splitext", |args| {
            match args.first() {
                Some(PyValue::Str(path)) => {
                    let p = std::path::Path::new(path.as_ref());
                    let stem =
                        p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let ext = p
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();

                    // Reconstruct the path without extension
                    let parent =
                        p.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                    let root = if parent.is_empty() {
                        stem
                    } else {
                        format!("{}{}{}", parent, std::path::MAIN_SEPARATOR, stem)
                    };

                    Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                        PyValue::Str(Arc::from(root)),
                        PyValue::Str(Arc::from(ext)),
                    ]))))
                }
                _ => Err(RuntimeError::type_error(
                    "str",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        PyBuiltinFunction::new("abspath", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                let p = std::path::Path::new(path.as_ref());
                let abs = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    std::env::current_dir().unwrap_or_default().join(p)
                };
                Ok(PyValue::Str(Arc::from(abs.to_string_lossy().to_string())))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        PyBuiltinFunction::new("normpath", |args| {
            match args.first() {
                Some(PyValue::Str(path)) => {
                    let p = std::path::Path::new(path.as_ref());
                    // Simplified normpath - just convert separators
                    let normalized = p.to_string_lossy().to_string();
                    Ok(PyValue::Str(Arc::from(normalized)))
                }
                _ => Err(RuntimeError::type_error(
                    "str",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        PyBuiltinFunction::new("expanduser", |args| {
            match args.first() {
                Some(PyValue::Str(path)) => {
                    let path_str = path.as_ref();
                    if path_str.starts_with('~') {
                        // Try to get home directory from environment
                        let home = if cfg!(target_os = "windows") {
                            std::env::var("USERPROFILE").ok()
                        } else {
                            std::env::var("HOME").ok()
                        };

                        if let Some(home_dir) = home {
                            let expanded = path_str.replacen('~', &home_dir, 1);
                            return Ok(PyValue::Str(Arc::from(expanded)));
                        }
                    }
                    Ok(PyValue::Str(Arc::clone(path)))
                }
                _ => Err(RuntimeError::type_error(
                    "str",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        PyBuiltinFunction::new("getsize", |args| match args.first() {
            Some(PyValue::Str(path)) => {
                let metadata =
                    std::fs::metadata(path.as_ref()).map_err(|e| RuntimeError::OsError {
                        message: format!("Cannot get size of '{}': {}", path, e),
                    })?;
                Ok(PyValue::Int(metadata.len() as i64))
            }
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
    ]
}

/// Create io module builtins
pub fn io_builtins() -> Vec<PyBuiltinFunction> {
    vec![PyBuiltinFunction::new("open", |args| {
        // Simplified open - just returns the filename for now
        match args.first() {
            Some(PyValue::Str(path)) => Ok(PyValue::Str(Arc::clone(path))),
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }
    })]
}

// ===== io module (Task 7.9) =====

/// Get the io module as a dict
pub fn io_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Default buffer size
    dict.setitem(PyKey::Str(Arc::from("DEFAULT_BUFFER_SIZE")), PyValue::Int(8192));

    // IO base classes (represented as strings for now)
    dict.setitem(PyKey::Str(Arc::from("IOBase")), PyValue::Str(Arc::from("<class 'io.IOBase'>")));
    dict.setitem(
        PyKey::Str(Arc::from("RawIOBase")),
        PyValue::Str(Arc::from("<class 'io.RawIOBase'>")),
    );
    dict.setitem(
        PyKey::Str(Arc::from("BufferedIOBase")),
        PyValue::Str(Arc::from("<class 'io.BufferedIOBase'>")),
    );
    dict.setitem(
        PyKey::Str(Arc::from("TextIOBase")),
        PyValue::Str(Arc::from("<class 'io.TextIOBase'>")),
    );

    Arc::new(dict)
}

/// Create io module builtins (expanded)
pub fn io_builtins_expanded() -> Vec<PyBuiltinFunction> {
    vec![
        // StringIO - in-memory text stream
        PyBuiltinFunction::new("StringIO", |args| {
            let initial = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(v) => return Err(RuntimeError::type_error("str or None", v.type_name())),
                None => String::new(),
            };

            let io_dict = PyDict::new();
            io_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("StringIO")));
            io_dict.setitem(PyKey::Str(Arc::from("_buffer")), PyValue::Str(Arc::from(initial)));
            io_dict.setitem(PyKey::Str(Arc::from("_pos")), PyValue::Int(0));
            io_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));

            Ok(PyValue::Dict(Arc::new(io_dict)))
        }),
        // BytesIO - in-memory binary stream
        PyBuiltinFunction::new("BytesIO", |args| {
            let initial = match args.first() {
                Some(PyValue::List(l)) => l.to_vec(),
                Some(v) => return Err(RuntimeError::type_error("bytes or None", v.type_name())),
                None => Vec::new(),
            };

            let io_dict = PyDict::new();
            io_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("BytesIO")));
            io_dict.setitem(
                PyKey::Str(Arc::from("_buffer")),
                PyValue::List(Arc::new(PyList::from_values(initial))),
            );
            io_dict.setitem(PyKey::Str(Arc::from("_pos")), PyValue::Int(0));
            io_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));

            Ok(PyValue::Dict(Arc::new(io_dict)))
        }),
        // StringIO.getvalue - get the entire contents
        PyBuiltinFunction::new("StringIO_getvalue", |args| match args.first() {
            Some(PyValue::Dict(d)) => d
                .getitem(&PyKey::Str(Arc::from("_buffer")))
                .map_err(|_| RuntimeError::attribute_error("StringIO", "_buffer")),
            _ => Err(RuntimeError::type_error(
                "StringIO",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // StringIO.write - write string to buffer
        PyBuiltinFunction::new("StringIO_write", |args| {
            if args.len() < 2 {
                return Err(RuntimeError::type_error(
                    "2 arguments",
                    format!("{} arguments", args.len()),
                ));
            }

            match (&args[0], &args[1]) {
                (PyValue::Dict(d), PyValue::Str(s)) => {
                    let current = d
                        .getitem(&PyKey::Str(Arc::from("_buffer")))
                        .map_err(|_| RuntimeError::attribute_error("StringIO", "_buffer"))?;

                    if let PyValue::Str(current_str) = current {
                        let new_buffer = format!("{}{}", current_str, s);
                        d.setitem(
                            PyKey::Str(Arc::from("_buffer")),
                            PyValue::Str(Arc::from(new_buffer)),
                        );
                        Ok(PyValue::Int(s.len() as i64))
                    } else {
                        Err(RuntimeError::internal_error("Invalid StringIO buffer"))
                    }
                }
                _ => Err(RuntimeError::type_error("StringIO and str", "other")),
            }
        }),
        // StringIO.read - read from buffer
        PyBuiltinFunction::new("StringIO_read", |args| {
            match args.first() {
                Some(PyValue::Dict(d)) => {
                    let buffer = d
                        .getitem(&PyKey::Str(Arc::from("_buffer")))
                        .map_err(|_| RuntimeError::attribute_error("StringIO", "_buffer"))?;
                    let pos = d
                        .getitem(&PyKey::Str(Arc::from("_pos")))
                        .map_err(|_| RuntimeError::attribute_error("StringIO", "_pos"))?;

                    if let (PyValue::Str(buf), PyValue::Int(p)) = (buffer, pos) {
                        let size = match args.get(1) {
                            Some(PyValue::Int(n)) if *n >= 0 => Some(*n as usize),
                            Some(PyValue::Int(_)) | None => None, // Read all
                            Some(v) => {
                                return Err(RuntimeError::type_error("int or None", v.type_name()))
                            }
                        };

                        let start = p as usize;
                        let content = &buf[start..];
                        let result = match size {
                            Some(n) => content.chars().take(n).collect::<String>(),
                            None => content.to_string(),
                        };

                        let new_pos = start + result.len();
                        d.setitem(PyKey::Str(Arc::from("_pos")), PyValue::Int(new_pos as i64));

                        Ok(PyValue::Str(Arc::from(result)))
                    } else {
                        Err(RuntimeError::internal_error("Invalid StringIO state"))
                    }
                }
                _ => Err(RuntimeError::type_error(
                    "StringIO",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        // StringIO.seek - move position
        PyBuiltinFunction::new("StringIO_seek", |args| {
            if args.len() < 2 {
                return Err(RuntimeError::type_error(
                    "2 arguments",
                    format!("{} arguments", args.len()),
                ));
            }

            match (&args[0], &args[1]) {
                (PyValue::Dict(d), PyValue::Int(pos)) => {
                    let whence = match args.get(2) {
                        Some(PyValue::Int(w)) => *w,
                        None => 0,
                        Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
                    };

                    let buffer = d
                        .getitem(&PyKey::Str(Arc::from("_buffer")))
                        .map_err(|_| RuntimeError::attribute_error("StringIO", "_buffer"))?;
                    let current_pos = d
                        .getitem(&PyKey::Str(Arc::from("_pos")))
                        .map_err(|_| RuntimeError::attribute_error("StringIO", "_pos"))?;

                    if let (PyValue::Str(buf), PyValue::Int(cur)) = (buffer, current_pos) {
                        let new_pos = match whence {
                            0 => *pos,                   // SEEK_SET
                            1 => cur + pos,              // SEEK_CUR
                            2 => buf.len() as i64 + pos, // SEEK_END
                            _ => return Err(RuntimeError::value_error("invalid whence value")),
                        };

                        let clamped_pos = new_pos.max(0).min(buf.len() as i64);
                        d.setitem(PyKey::Str(Arc::from("_pos")), PyValue::Int(clamped_pos));

                        Ok(PyValue::Int(clamped_pos))
                    } else {
                        Err(RuntimeError::internal_error("Invalid StringIO state"))
                    }
                }
                _ => Err(RuntimeError::type_error("StringIO and int", "other")),
            }
        }),
        // StringIO.tell - get current position
        PyBuiltinFunction::new("StringIO_tell", |args| match args.first() {
            Some(PyValue::Dict(d)) => d
                .getitem(&PyKey::Str(Arc::from("_pos")))
                .map_err(|_| RuntimeError::attribute_error("StringIO", "_pos")),
            _ => Err(RuntimeError::type_error(
                "StringIO",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // StringIO.close - close the stream
        PyBuiltinFunction::new("StringIO_close", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(true));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "StringIO",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // BytesIO.getvalue - get the entire contents
        PyBuiltinFunction::new("BytesIO_getvalue", |args| match args.first() {
            Some(PyValue::Dict(d)) => d
                .getitem(&PyKey::Str(Arc::from("_buffer")))
                .map_err(|_| RuntimeError::attribute_error("BytesIO", "_buffer")),
            _ => Err(RuntimeError::type_error(
                "BytesIO",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // BytesIO.write - write bytes to buffer
        PyBuiltinFunction::new("BytesIO_write", |args| {
            if args.len() < 2 {
                return Err(RuntimeError::type_error(
                    "2 arguments",
                    format!("{} arguments", args.len()),
                ));
            }

            match (&args[0], &args[1]) {
                (PyValue::Dict(d), PyValue::List(bytes)) => {
                    let current = d
                        .getitem(&PyKey::Str(Arc::from("_buffer")))
                        .map_err(|_| RuntimeError::attribute_error("BytesIO", "_buffer"))?;

                    if let PyValue::List(current_list) = current {
                        let mut new_buffer = current_list.to_vec();
                        new_buffer.extend(bytes.to_vec());
                        d.setitem(
                            PyKey::Str(Arc::from("_buffer")),
                            PyValue::List(Arc::new(PyList::from_values(new_buffer))),
                        );
                        Ok(PyValue::Int(bytes.len() as i64))
                    } else {
                        Err(RuntimeError::internal_error("Invalid BytesIO buffer"))
                    }
                }
                _ => Err(RuntimeError::type_error("BytesIO and bytes", "other")),
            }
        }),
        // BytesIO.read - read from buffer
        PyBuiltinFunction::new("BytesIO_read", |args| {
            match args.first() {
                Some(PyValue::Dict(d)) => {
                    let buffer = d
                        .getitem(&PyKey::Str(Arc::from("_buffer")))
                        .map_err(|_| RuntimeError::attribute_error("BytesIO", "_buffer"))?;
                    let pos = d
                        .getitem(&PyKey::Str(Arc::from("_pos")))
                        .map_err(|_| RuntimeError::attribute_error("BytesIO", "_pos"))?;

                    if let (PyValue::List(buf), PyValue::Int(p)) = (buffer, pos) {
                        let size = match args.get(1) {
                            Some(PyValue::Int(n)) if *n >= 0 => Some(*n as usize),
                            Some(PyValue::Int(_)) | None => None, // Read all
                            Some(v) => {
                                return Err(RuntimeError::type_error("int or None", v.type_name()))
                            }
                        };

                        let start = p as usize;
                        let content = buf.to_vec();
                        let remaining = &content[start.min(content.len())..];
                        let result: Vec<PyValue> = match size {
                            Some(n) => remaining.iter().take(n).cloned().collect(),
                            None => remaining.to_vec(),
                        };

                        let new_pos = start + result.len();
                        d.setitem(PyKey::Str(Arc::from("_pos")), PyValue::Int(new_pos as i64));

                        Ok(PyValue::List(Arc::new(PyList::from_values(result))))
                    } else {
                        Err(RuntimeError::internal_error("Invalid BytesIO state"))
                    }
                }
                _ => Err(RuntimeError::type_error(
                    "BytesIO",
                    args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        // TextIOWrapper - wrap a binary stream with text encoding
        PyBuiltinFunction::new("TextIOWrapper", |args| {
            let buffer = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("buffer", "nothing")),
            };

            let encoding = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::None) | None => "utf-8".to_string(),
                Some(v) => return Err(RuntimeError::type_error("str or None", v.type_name())),
            };

            let io_dict = PyDict::new();
            io_dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("TextIOWrapper")),
            );
            io_dict.setitem(PyKey::Str(Arc::from("_buffer")), buffer);
            io_dict.setitem(PyKey::Str(Arc::from("_encoding")), PyValue::Str(Arc::from(encoding)));
            io_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));

            Ok(PyValue::Dict(Arc::new(io_dict)))
        }),
        // BufferedReader - buffered binary reader
        PyBuiltinFunction::new("BufferedReader", |args| {
            let raw = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("raw stream", "nothing")),
            };

            let buffer_size = match args.get(1) {
                Some(PyValue::Int(n)) => *n,
                None => 8192,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let io_dict = PyDict::new();
            io_dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("BufferedReader")),
            );
            io_dict.setitem(PyKey::Str(Arc::from("_raw")), raw);
            io_dict.setitem(PyKey::Str(Arc::from("_buffer_size")), PyValue::Int(buffer_size));
            io_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));

            Ok(PyValue::Dict(Arc::new(io_dict)))
        }),
        // BufferedWriter - buffered binary writer
        PyBuiltinFunction::new("BufferedWriter", |args| {
            let raw = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("raw stream", "nothing")),
            };

            let buffer_size = match args.get(1) {
                Some(PyValue::Int(n)) => *n,
                None => 8192,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let io_dict = PyDict::new();
            io_dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("BufferedWriter")),
            );
            io_dict.setitem(PyKey::Str(Arc::from("_raw")), raw);
            io_dict.setitem(PyKey::Str(Arc::from("_buffer_size")), PyValue::Int(buffer_size));
            io_dict
                .setitem(PyKey::Str(Arc::from("_buffer")), PyValue::List(Arc::new(PyList::new())));
            io_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));

            Ok(PyValue::Dict(Arc::new(io_dict)))
        }),
    ]
}

/// Create json module builtins (using simd-json when available)
pub fn json_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        PyBuiltinFunction::new("dumps", |args| match args.first() {
            Some(value) => {
                let json = value_to_json(value)?;
                Ok(PyValue::Str(Arc::from(json)))
            }
            None => Err(RuntimeError::type_error("1 argument", "0 arguments")),
        }),
        PyBuiltinFunction::new("loads", |args| match args.first() {
            Some(PyValue::Str(s)) => json_to_value(s),
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
    ]
}

/// Convert PyValue to JSON string
fn value_to_json(value: &PyValue) -> RuntimeResult<String> {
    match value {
        PyValue::None => Ok("null".to_string()),
        PyValue::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        PyValue::Int(i) => Ok(i.to_string()),
        PyValue::Float(f) => Ok(f.to_string()),
        PyValue::Str(s) => Ok(format!("\"{}\"", escape_json_string(s))),
        PyValue::List(list) => {
            let items: RuntimeResult<Vec<String>> =
                list.to_vec().iter().map(value_to_json).collect();
            Ok(format!("[{}]", items?.join(",")))
        }
        PyValue::Set(set) => {
            // JSON doesn't have sets, serialize as array
            let items: RuntimeResult<Vec<String>> =
                set.to_vec().iter().map(value_to_json).collect();
            Ok(format!("[{}]", items?.join(",")))
        }
        PyValue::Dict(dict) => {
            let items: RuntimeResult<Vec<String>> = dict
                .items()
                .iter()
                .map(|(k, v)| {
                    let key_str = match k {
                        crate::pydict::PyKey::Str(s) => format!("\"{}\"", escape_json_string(s)),
                        crate::pydict::PyKey::Int(i) => format!("\"{}\"", i),
                        _ => return Err(RuntimeError::type_error("str key", "non-str key")),
                    };
                    let val_str = value_to_json(v)?;
                    Ok(format!("{}:{}", key_str, val_str))
                })
                .collect();
            Ok(format!("{{{}}}", items?.join(",")))
        }
        PyValue::Tuple(tuple) => {
            let items: RuntimeResult<Vec<String>> =
                tuple.to_vec().iter().map(value_to_json).collect();
            Ok(format!("[{}]", items?.join(",")))
        }
        PyValue::Exception(e) => {
            // Serialize exception as a dict with type and message
            Ok(format!(
                "{{\"type\":\"{}\",\"message\":\"{}\"}}",
                escape_json_string(&e.exc_type),
                escape_json_string(&e.message)
            ))
        }
        PyValue::Type(t) => {
            // Serialize type as a string representation
            Ok(format!("\"<class '{}'>\"", escape_json_string(&t.name)))
        }
        PyValue::Instance(inst) => {
            // Serialize instance as a string representation
            Ok(format!("\"<{} object>\"", escape_json_string(&inst.class.name)))
        }
        PyValue::BoundMethod(_) => {
            // Serialize bound method as a string representation
            Ok("\"<bound method>\"".to_string())
        }
        PyValue::Generator(gen) => {
            // Serialize generator as a string representation
            Ok(format!("\"<generator object {}>\"", escape_json_string(&gen.name)))
        }
        PyValue::Coroutine(coro) => {
            // Serialize coroutine as a string representation
            Ok(format!("\"<coroutine object {}>\"", escape_json_string(&coro.name)))
        }
        PyValue::Builtin(b) => {
            Ok(format!("\"<built-in function {}>\"", escape_json_string(&b.name)))
        }
        PyValue::Function(f) => Ok(format!("\"<function {}>\"", escape_json_string(&f.name))),
        PyValue::Iterator(_) => Ok("\"<iterator>\"".to_string()),
        PyValue::Module(m) => Ok(format!("\"<module '{}'>\"", escape_json_string(&m.name))),
        PyValue::Code(c) => Ok(format!("\"<code object {}>\"", escape_json_string(&c.name))),
        PyValue::Cell(cell) => {
            // Serialize the cell's contents
            value_to_json(&cell.get())
        }
        PyValue::Super(s) => {
            Ok(format!("\"<super: <class '{}'>>\"", escape_json_string(&s.type_.name)))
        }
        PyValue::Property(p) => Ok(format!(
            "\"<property: {}>\"",
            escape_json_string(p.get_doc().unwrap_or("no doc"))
        )),
        PyValue::StaticMethod(_) => Ok("\"<staticmethod>\"".to_string()),
        PyValue::ClassMethod(_) => Ok("\"<classmethod>\"".to_string()),
    }
}

/// Escape special characters in JSON string
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// Parse JSON string to PyValue (uses the full parser with JSONDecodeError)
fn json_to_value(s: &str) -> RuntimeResult<PyValue> {
    json_to_value_nested(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_dumps_primitives() {
        assert_eq!(value_to_json(&PyValue::None).unwrap(), "null");
        assert_eq!(value_to_json(&PyValue::Bool(true)).unwrap(), "true");
        assert_eq!(value_to_json(&PyValue::Int(42)).unwrap(), "42");
        assert_eq!(value_to_json(&PyValue::Str(Arc::from("hello"))).unwrap(), "\"hello\"");
    }

    #[test]
    fn test_json_loads_primitives() {
        assert!(matches!(json_to_value("null").unwrap(), PyValue::None));
        assert!(matches!(json_to_value("true").unwrap(), PyValue::Bool(true)));
        assert!(matches!(json_to_value("42").unwrap(), PyValue::Int(42)));
    }

    #[test]
    fn test_json_roundtrip() {
        let original = PyValue::Int(123);
        let json = value_to_json(&original).unwrap();
        let parsed = json_to_value(&json).unwrap();

        if let (PyValue::Int(a), PyValue::Int(b)) = (&original, &parsed) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_os_getcwd() {
        let builtins = os_builtins();
        let getcwd = builtins.iter().find(|f| f.name == "getcwd").unwrap();

        let result = getcwd.call(&[]).unwrap();
        assert!(matches!(result, PyValue::Str(_)));
    }

    // ===== sys module tests (Task 7.6) =====

    #[test]
    fn test_sys_module_version() {
        let sys = sys_module();
        let version = sys.getitem(&PyKey::Str(Arc::from("version"))).unwrap();
        if let PyValue::Str(s) = version {
            assert!(s.contains("3.12"));
            assert!(s.contains("dx-py"));
        } else {
            panic!("Expected string for version");
        }
    }

    #[test]
    fn test_sys_module_version_info() {
        let sys = sys_module();
        let version_info = sys.getitem(&PyKey::Str(Arc::from("version_info"))).unwrap();
        if let PyValue::Tuple(t) = version_info {
            assert_eq!(t.len(), 5);
            if let PyValue::Int(major) = &t.to_vec()[0] {
                assert_eq!(*major, 3);
            }
        } else {
            panic!("Expected tuple for version_info");
        }
    }

    #[test]
    fn test_sys_module_platform() {
        let sys = sys_module();
        let platform = sys.getitem(&PyKey::Str(Arc::from("platform"))).unwrap();
        if let PyValue::Str(s) = platform {
            assert!(["win32", "darwin", "linux", "unknown"].contains(&s.as_ref()));
        } else {
            panic!("Expected string for platform");
        }
    }

    #[test]
    fn test_sys_module_path() {
        let sys = sys_module();
        let path = sys.getitem(&PyKey::Str(Arc::from("path"))).unwrap();
        assert!(matches!(path, PyValue::List(_)));
    }

    #[test]
    fn test_sys_module_maxsize() {
        let sys = sys_module();
        let maxsize = sys.getitem(&PyKey::Str(Arc::from("maxsize"))).unwrap();
        if let PyValue::Int(i) = maxsize {
            assert_eq!(i, i64::MAX);
        } else {
            panic!("Expected int for maxsize");
        }
    }

    #[test]
    fn test_sys_exit() {
        let builtins = sys_builtins();
        let exit = builtins.iter().find(|f| f.name == "exit").unwrap();

        // Exit with code 0
        let result = exit.call(&[PyValue::Int(0)]);
        assert!(result.is_err());

        // Exit with no args
        let result = exit.call(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sys_getrecursionlimit() {
        let builtins = sys_builtins();
        let getrecursionlimit = builtins.iter().find(|f| f.name == "getrecursionlimit").unwrap();

        let result = getrecursionlimit.call(&[]).unwrap();
        if let PyValue::Int(limit) = result {
            assert!(limit > 0);
        } else {
            panic!("Expected int");
        }
    }

    #[test]
    fn test_sys_getsizeof() {
        let builtins = sys_builtins();
        let getsizeof = builtins.iter().find(|f| f.name == "getsizeof").unwrap();

        let result = getsizeof.call(&[PyValue::Int(42)]).unwrap();
        if let PyValue::Int(size) = result {
            assert!(size > 0);
        } else {
            panic!("Expected int");
        }
    }

    #[test]
    fn test_sys_getdefaultencoding() {
        let builtins = sys_builtins();
        let getdefaultencoding = builtins.iter().find(|f| f.name == "getdefaultencoding").unwrap();

        let result = getdefaultencoding.call(&[]).unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "utf-8");
        } else {
            panic!("Expected string");
        }
    }

    // ===== os module tests (Task 7.7) =====

    #[test]
    fn test_os_module_name() {
        let os = os_module();
        let name = os.getitem(&PyKey::Str(Arc::from("name"))).unwrap();
        if let PyValue::Str(s) = name {
            assert!(["nt", "posix"].contains(&s.as_ref()));
        } else {
            panic!("Expected string for name");
        }
    }

    #[test]
    fn test_os_module_sep() {
        let os = os_module();
        let sep = os.getitem(&PyKey::Str(Arc::from("sep"))).unwrap();
        if let PyValue::Str(s) = sep {
            assert!(s.len() == 1);
        } else {
            panic!("Expected string for sep");
        }
    }

    #[test]
    fn test_os_module_environ() {
        let os = os_module();
        let environ = os.getitem(&PyKey::Str(Arc::from("environ"))).unwrap();
        assert!(matches!(environ, PyValue::Dict(_)));
    }

    #[test]
    fn test_os_chdir_getcwd() {
        let builtins = os_builtins();
        let getcwd = builtins.iter().find(|f| f.name == "getcwd").unwrap();

        let result = getcwd.call(&[]).unwrap();
        assert!(matches!(result, PyValue::Str(_)));
    }

    #[test]
    fn test_os_getpid() {
        let builtins = os_builtins();
        let getpid = builtins.iter().find(|f| f.name == "getpid").unwrap();

        let result = getpid.call(&[]).unwrap();
        if let PyValue::Int(pid) = result {
            assert!(pid > 0);
        } else {
            panic!("Expected int");
        }
    }

    #[test]
    fn test_os_cpu_count() {
        let builtins = os_builtins();
        let cpu_count = builtins.iter().find(|f| f.name == "cpu_count").unwrap();

        let result = cpu_count.call(&[]).unwrap();
        if let PyValue::Int(count) = result {
            assert!(count >= 1);
        } else {
            panic!("Expected int");
        }
    }

    #[test]
    fn test_os_path_join() {
        let builtins = os_path_builtins();
        let join = builtins.iter().find(|f| f.name == "join").unwrap();

        let result = join
            .call(&[
                PyValue::Str(Arc::from("a")),
                PyValue::Str(Arc::from("b")),
                PyValue::Str(Arc::from("c")),
            ])
            .unwrap();

        if let PyValue::Str(s) = result {
            assert!(s.contains("a"));
            assert!(s.contains("b"));
            assert!(s.contains("c"));
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_os_path_exists() {
        let builtins = os_path_builtins();
        let exists = builtins.iter().find(|f| f.name == "exists").unwrap();

        // Current directory should exist
        let result = exists.call(&[PyValue::Str(Arc::from("."))]).unwrap();
        assert!(matches!(result, PyValue::Bool(true)));

        // Non-existent path
        let result = exists.call(&[PyValue::Str(Arc::from("/nonexistent/path/12345"))]).unwrap();
        assert!(matches!(result, PyValue::Bool(false)));
    }

    #[test]
    fn test_os_path_isdir() {
        let builtins = os_path_builtins();
        let isdir = builtins.iter().find(|f| f.name == "isdir").unwrap();

        // Current directory should be a directory
        let result = isdir.call(&[PyValue::Str(Arc::from("."))]).unwrap();
        assert!(matches!(result, PyValue::Bool(true)));
    }

    #[test]
    fn test_os_path_dirname_basename() {
        let builtins = os_path_builtins();
        let dirname = builtins.iter().find(|f| f.name == "dirname").unwrap();
        let basename = builtins.iter().find(|f| f.name == "basename").unwrap();

        let path = if cfg!(target_os = "windows") {
            "C:\\foo\\bar\\baz.txt"
        } else {
            "/foo/bar/baz.txt"
        };

        let dir_result = dirname.call(&[PyValue::Str(Arc::from(path))]).unwrap();
        if let PyValue::Str(s) = dir_result {
            assert!(s.contains("bar"));
        }

        let base_result = basename.call(&[PyValue::Str(Arc::from(path))]).unwrap();
        if let PyValue::Str(s) = base_result {
            assert_eq!(s.as_ref(), "baz.txt");
        }
    }

    #[test]
    fn test_os_path_split() {
        let builtins = os_path_builtins();
        let split = builtins.iter().find(|f| f.name == "split").unwrap();

        let path = if cfg!(target_os = "windows") {
            "C:\\foo\\bar.txt"
        } else {
            "/foo/bar.txt"
        };

        let result = split.call(&[PyValue::Str(Arc::from(path))]).unwrap();
        if let PyValue::Tuple(t) = result {
            assert_eq!(t.len(), 2);
            if let PyValue::Str(basename) = &t.to_vec()[1] {
                assert_eq!(basename.as_ref(), "bar.txt");
            }
        } else {
            panic!("Expected tuple");
        }
    }

    #[test]
    fn test_os_path_splitext() {
        let builtins = os_path_builtins();
        let splitext = builtins.iter().find(|f| f.name == "splitext").unwrap();

        let result = splitext.call(&[PyValue::Str(Arc::from("file.txt"))]).unwrap();
        if let PyValue::Tuple(t) = result {
            assert_eq!(t.len(), 2);
            if let PyValue::Str(ext) = &t.to_vec()[1] {
                assert_eq!(ext.as_ref(), ".txt");
            }
        } else {
            panic!("Expected tuple");
        }
    }

    #[test]
    fn test_os_path_isabs() {
        let builtins = os_path_builtins();
        let isabs = builtins.iter().find(|f| f.name == "isabs").unwrap();

        // Relative path
        let result = isabs.call(&[PyValue::Str(Arc::from("relative/path"))]).unwrap();
        assert!(matches!(result, PyValue::Bool(false)));

        // Absolute path
        let abs_path = if cfg!(target_os = "windows") {
            "C:\\absolute\\path"
        } else {
            "/absolute/path"
        };
        let result = isabs.call(&[PyValue::Str(Arc::from(abs_path))]).unwrap();
        assert!(matches!(result, PyValue::Bool(true)));
    }

    // ===== io module tests (Task 7.9) =====

    #[test]
    fn test_io_module_attributes() {
        let io = io_module();

        let buffer_size = io.getitem(&PyKey::Str(Arc::from("DEFAULT_BUFFER_SIZE"))).unwrap();
        if let PyValue::Int(size) = buffer_size {
            assert_eq!(size, 8192);
        } else {
            panic!("Expected int for DEFAULT_BUFFER_SIZE");
        }
    }

    #[test]
    fn test_stringio_create() {
        let builtins = io_builtins_expanded();
        let stringio = builtins.iter().find(|f| f.name == "StringIO").unwrap();

        // Create empty StringIO
        let result = stringio.call(&[]).unwrap();
        assert!(matches!(result, PyValue::Dict(_)));

        // Create with initial value
        let result = stringio.call(&[PyValue::Str(Arc::from("hello"))]).unwrap();
        if let PyValue::Dict(d) = result {
            let buffer = d.getitem(&PyKey::Str(Arc::from("_buffer"))).unwrap();
            if let PyValue::Str(s) = buffer {
                assert_eq!(s.as_ref(), "hello");
            }
        }
    }

    #[test]
    fn test_stringio_write_getvalue() {
        let builtins = io_builtins_expanded();
        let stringio = builtins.iter().find(|f| f.name == "StringIO").unwrap();
        let write = builtins.iter().find(|f| f.name == "StringIO_write").unwrap();
        let getvalue = builtins.iter().find(|f| f.name == "StringIO_getvalue").unwrap();

        // Create StringIO
        let io = stringio.call(&[]).unwrap();

        // Write to it
        let written = write.call(&[io.clone(), PyValue::Str(Arc::from("hello"))]).unwrap();
        if let PyValue::Int(n) = written {
            assert_eq!(n, 5);
        }

        // Write more
        write.call(&[io.clone(), PyValue::Str(Arc::from(" world"))]).unwrap();

        // Get value
        let value = getvalue.call(&[io]).unwrap();
        if let PyValue::Str(s) = value {
            assert_eq!(s.as_ref(), "hello world");
        }
    }

    #[test]
    fn test_stringio_read() {
        let builtins = io_builtins_expanded();
        let stringio = builtins.iter().find(|f| f.name == "StringIO").unwrap();
        let read = builtins.iter().find(|f| f.name == "StringIO_read").unwrap();

        // Create StringIO with content
        let io = stringio.call(&[PyValue::Str(Arc::from("hello world"))]).unwrap();

        // Read all
        let result = read.call(&[io.clone()]).unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(s.as_ref(), "hello world");
        }
    }

    #[test]
    fn test_stringio_seek_tell() {
        let builtins = io_builtins_expanded();
        let stringio = builtins.iter().find(|f| f.name == "StringIO").unwrap();
        let seek = builtins.iter().find(|f| f.name == "StringIO_seek").unwrap();
        let tell = builtins.iter().find(|f| f.name == "StringIO_tell").unwrap();
        let read = builtins.iter().find(|f| f.name == "StringIO_read").unwrap();

        // Create StringIO with content
        let io = stringio.call(&[PyValue::Str(Arc::from("hello world"))]).unwrap();

        // Read to move position
        read.call(&[io.clone()]).unwrap();

        // Tell should be at end
        let pos = tell.call(&[io.clone()]).unwrap();
        if let PyValue::Int(p) = pos {
            assert_eq!(p, 11);
        }

        // Seek to beginning
        seek.call(&[io.clone(), PyValue::Int(0)]).unwrap();

        // Tell should be at beginning
        let pos = tell.call(&[io.clone()]).unwrap();
        if let PyValue::Int(p) = pos {
            assert_eq!(p, 0);
        }
    }

    #[test]
    fn test_bytesio_create() {
        let builtins = io_builtins_expanded();
        let bytesio = builtins.iter().find(|f| f.name == "BytesIO").unwrap();

        // Create empty BytesIO
        let result = bytesio.call(&[]).unwrap();
        assert!(matches!(result, PyValue::Dict(_)));
    }

    #[test]
    fn test_bytesio_write_getvalue() {
        let builtins = io_builtins_expanded();
        let bytesio = builtins.iter().find(|f| f.name == "BytesIO").unwrap();
        let write = builtins.iter().find(|f| f.name == "BytesIO_write").unwrap();
        let getvalue = builtins.iter().find(|f| f.name == "BytesIO_getvalue").unwrap();

        // Create BytesIO
        let io = bytesio.call(&[]).unwrap();

        // Write bytes
        let bytes = PyValue::List(Arc::new(PyList::from_values(vec![
            PyValue::Int(72),  // H
            PyValue::Int(105), // i
        ])));
        let written = write.call(&[io.clone(), bytes]).unwrap();
        if let PyValue::Int(n) = written {
            assert_eq!(n, 2);
        }

        // Get value
        let value = getvalue.call(&[io]).unwrap();
        if let PyValue::List(l) = value {
            assert_eq!(l.len(), 2);
        }
    }

    #[test]
    fn test_textiowrapper_create() {
        let builtins = io_builtins_expanded();
        let bytesio = builtins.iter().find(|f| f.name == "BytesIO").unwrap();
        let textiowrapper = builtins.iter().find(|f| f.name == "TextIOWrapper").unwrap();

        // Create BytesIO as buffer
        let buffer = bytesio.call(&[]).unwrap();

        // Wrap with TextIOWrapper
        let result = textiowrapper.call(&[buffer, PyValue::Str(Arc::from("utf-8"))]).unwrap();

        if let PyValue::Dict(d) = result {
            let class = d.getitem(&PyKey::Str(Arc::from("__class__"))).unwrap();
            if let PyValue::Str(s) = class {
                assert_eq!(s.as_ref(), "TextIOWrapper");
            }
        }
    }

    #[test]
    fn test_buffered_reader_create() {
        let builtins = io_builtins_expanded();
        let bytesio = builtins.iter().find(|f| f.name == "BytesIO").unwrap();
        let buffered_reader = builtins.iter().find(|f| f.name == "BufferedReader").unwrap();

        // Create BytesIO as raw stream
        let raw = bytesio.call(&[]).unwrap();

        // Create BufferedReader
        let result = buffered_reader.call(&[raw]).unwrap();

        if let PyValue::Dict(d) = result {
            let class = d.getitem(&PyKey::Str(Arc::from("__class__"))).unwrap();
            if let PyValue::Str(s) = class {
                assert_eq!(s.as_ref(), "BufferedReader");
            }
        }
    }

    #[test]
    fn test_buffered_writer_create() {
        let builtins = io_builtins_expanded();
        let bytesio = builtins.iter().find(|f| f.name == "BytesIO").unwrap();
        let buffered_writer = builtins.iter().find(|f| f.name == "BufferedWriter").unwrap();

        // Create BytesIO as raw stream
        let raw = bytesio.call(&[]).unwrap();

        // Create BufferedWriter with custom buffer size
        let result = buffered_writer.call(&[raw, PyValue::Int(4096)]).unwrap();

        if let PyValue::Dict(d) = result {
            let buffer_size = d.getitem(&PyKey::Str(Arc::from("_buffer_size"))).unwrap();
            if let PyValue::Int(size) = buffer_size {
                assert_eq!(size, 4096);
            }
        }
    }
}

// ===== collections module (Task 7.10) =====

/// Get the collections module as a dict
pub fn collections_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("collections")));

    Arc::new(dict)
}

/// Create collections module builtins
pub fn collections_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // OrderedDict - dict that remembers insertion order
        // Note: In Python 3.7+, regular dicts maintain insertion order,
        // but OrderedDict has additional methods like move_to_end
        PyBuiltinFunction::new("OrderedDict", |args| {
            let dict = PyDict::new();
            dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("OrderedDict")),
            );
            dict.setitem(PyKey::Str(Arc::from("_data")), PyValue::Dict(Arc::new(PyDict::new())));
            dict.setitem(PyKey::Str(Arc::from("_order")), PyValue::List(Arc::new(PyList::new())));

            // Initialize from iterable if provided
            if let Some(PyValue::Dict(init_dict)) = args.first() {
                for (k, v) in init_dict.items() {
                    if let PyValue::Dict(d) = dict.getitem(&PyKey::Str(Arc::from("_data"))).unwrap()
                    {
                        d.setitem(k.clone(), v);
                    }
                    if let PyValue::List(order) =
                        dict.getitem(&PyKey::Str(Arc::from("_order"))).unwrap()
                    {
                        // Add key to order list
                        let mut items = order.to_vec();
                        items.push(k.to_value());
                        dict.setitem(
                            PyKey::Str(Arc::from("_order")),
                            PyValue::List(Arc::new(PyList::from_values(items))),
                        );
                    }
                }
            }

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // defaultdict - dict with default factory
        PyBuiltinFunction::new("defaultdict", |args| {
            let default_factory = match args.first() {
                Some(v) => v.clone(),
                None => PyValue::None,
            };

            let dict = PyDict::new();
            dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("defaultdict")),
            );
            dict.setitem(PyKey::Str(Arc::from("_data")), PyValue::Dict(Arc::new(PyDict::new())));
            dict.setitem(PyKey::Str(Arc::from("default_factory")), default_factory);

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // Counter - dict subclass for counting hashable objects
        PyBuiltinFunction::new("Counter", |args| {
            let dict = PyDict::new();
            dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Counter")));
            dict.setitem(PyKey::Str(Arc::from("_data")), PyValue::Dict(Arc::new(PyDict::new())));

            // Initialize from iterable if provided
            if let Some(iterable) = args.first() {
                let items = match iterable {
                    PyValue::List(l) => l.to_vec(),
                    PyValue::Tuple(t) => t.to_vec(),
                    PyValue::Str(s) => {
                        s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                    }
                    PyValue::Dict(d) => {
                        // Copy counts from dict
                        if let PyValue::Dict(data) =
                            dict.getitem(&PyKey::Str(Arc::from("_data"))).unwrap()
                        {
                            for (k, v) in d.items() {
                                data.setitem(k, v);
                            }
                        }
                        return Ok(PyValue::Dict(Arc::new(dict)));
                    }
                    _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
                };

                // Count items
                if let PyValue::Dict(data) = dict.getitem(&PyKey::Str(Arc::from("_data"))).unwrap()
                {
                    for item in items {
                        let key = PyKey::from_value(&item)?;
                        let current = data.get(&key, PyValue::Int(0));
                        if let PyValue::Int(count) = current {
                            data.setitem(key, PyValue::Int(count + 1));
                        }
                    }
                }
            }

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // Counter.most_common - return list of (elem, count) pairs
        PyBuiltinFunction::new("Counter_most_common", |args| {
            let counter = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Counter",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let n = match args.get(1) {
                Some(PyValue::Int(n)) => Some(*n as usize),
                Some(PyValue::None) | None => None,
                Some(v) => return Err(RuntimeError::type_error("int or None", v.type_name())),
            };

            let data = counter
                .getitem(&PyKey::Str(Arc::from("_data")))
                .map_err(|_| RuntimeError::attribute_error("Counter", "_data"))?;

            if let PyValue::Dict(d) = data {
                let mut items: Vec<(PyKey, i64)> = d
                    .items()
                    .into_iter()
                    .filter_map(|(k, v)| {
                        if let PyValue::Int(count) = v {
                            Some((k, count))
                        } else {
                            None
                        }
                    })
                    .collect();

                // Sort by count descending
                items.sort_by(|a, b| b.1.cmp(&a.1));

                // Take n items if specified
                let result_items: Vec<PyValue> = items
                    .into_iter()
                    .take(n.unwrap_or(usize::MAX))
                    .map(|(k, count)| {
                        PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                            k.to_value(),
                            PyValue::Int(count),
                        ])))
                    })
                    .collect();

                Ok(PyValue::List(Arc::new(PyList::from_values(result_items))))
            } else {
                Err(RuntimeError::internal_error("Invalid Counter data"))
            }
        }),
        // deque - double-ended queue
        PyBuiltinFunction::new("deque", |args| {
            let maxlen = match args.get(1) {
                Some(PyValue::Int(n)) if *n >= 0 => Some(*n as usize),
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("maxlen must be non-negative"))
                }
                Some(PyValue::None) | None => None,
                Some(v) => return Err(RuntimeError::type_error("int or None", v.type_name())),
            };

            let dict = PyDict::new();
            dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("deque")));
            dict.setitem(PyKey::Str(Arc::from("_data")), PyValue::List(Arc::new(PyList::new())));
            dict.setitem(
                PyKey::Str(Arc::from("maxlen")),
                match maxlen {
                    Some(n) => PyValue::Int(n as i64),
                    None => PyValue::None,
                },
            );

            // Initialize from iterable if provided
            if let Some(iterable) = args.first() {
                let items = match iterable {
                    PyValue::List(l) => l.to_vec(),
                    PyValue::Tuple(t) => t.to_vec(),
                    PyValue::Str(s) => {
                        s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                    }
                    _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
                };

                // Apply maxlen if set
                let final_items = match maxlen {
                    Some(n) if items.len() > n => items[items.len() - n..].to_vec(),
                    _ => items,
                };

                dict.setitem(
                    PyKey::Str(Arc::from("_data")),
                    PyValue::List(Arc::new(PyList::from_values(final_items))),
                );
            }

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // deque.append - add to right end
        PyBuiltinFunction::new("deque_append", |args| {
            if args.len() < 2 {
                return Err(RuntimeError::type_error(
                    "2 arguments",
                    format!("{} arguments", args.len()),
                ));
            }

            let deque = match &args[0] {
                PyValue::Dict(d) => d,
                _ => return Err(RuntimeError::type_error("deque", args[0].type_name())),
            };

            let item = args[1].clone();

            let data = deque
                .getitem(&PyKey::Str(Arc::from("_data")))
                .map_err(|_| RuntimeError::attribute_error("deque", "_data"))?;
            let maxlen = deque
                .getitem(&PyKey::Str(Arc::from("maxlen")))
                .map_err(|_| RuntimeError::attribute_error("deque", "maxlen"))?;

            if let PyValue::List(l) = data {
                let mut items = l.to_vec();
                items.push(item);

                // Enforce maxlen
                if let PyValue::Int(max) = maxlen {
                    while items.len() > max as usize {
                        items.remove(0);
                    }
                }

                deque.setitem(
                    PyKey::Str(Arc::from("_data")),
                    PyValue::List(Arc::new(PyList::from_values(items))),
                );
            }

            Ok(PyValue::None)
        }),
        // deque.appendleft - add to left end
        PyBuiltinFunction::new("deque_appendleft", |args| {
            if args.len() < 2 {
                return Err(RuntimeError::type_error(
                    "2 arguments",
                    format!("{} arguments", args.len()),
                ));
            }

            let deque = match &args[0] {
                PyValue::Dict(d) => d,
                _ => return Err(RuntimeError::type_error("deque", args[0].type_name())),
            };

            let item = args[1].clone();

            let data = deque
                .getitem(&PyKey::Str(Arc::from("_data")))
                .map_err(|_| RuntimeError::attribute_error("deque", "_data"))?;
            let maxlen = deque
                .getitem(&PyKey::Str(Arc::from("maxlen")))
                .map_err(|_| RuntimeError::attribute_error("deque", "maxlen"))?;

            if let PyValue::List(l) = data {
                let mut items = l.to_vec();
                items.insert(0, item);

                // Enforce maxlen
                if let PyValue::Int(max) = maxlen {
                    while items.len() > max as usize {
                        items.pop();
                    }
                }

                deque.setitem(
                    PyKey::Str(Arc::from("_data")),
                    PyValue::List(Arc::new(PyList::from_values(items))),
                );
            }

            Ok(PyValue::None)
        }),
        // deque.pop - remove and return from right end
        PyBuiltinFunction::new("deque_pop", |args| {
            let deque = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "deque",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let data = deque
                .getitem(&PyKey::Str(Arc::from("_data")))
                .map_err(|_| RuntimeError::attribute_error("deque", "_data"))?;

            if let PyValue::List(l) = data {
                let mut items = l.to_vec();
                if items.is_empty() {
                    return Err(RuntimeError::value_error("pop from an empty deque"));
                }
                let item = items.pop().unwrap();
                deque.setitem(
                    PyKey::Str(Arc::from("_data")),
                    PyValue::List(Arc::new(PyList::from_values(items))),
                );
                Ok(item)
            } else {
                Err(RuntimeError::internal_error("Invalid deque data"))
            }
        }),
        // deque.popleft - remove and return from left end
        PyBuiltinFunction::new("deque_popleft", |args| {
            let deque = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "deque",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let data = deque
                .getitem(&PyKey::Str(Arc::from("_data")))
                .map_err(|_| RuntimeError::attribute_error("deque", "_data"))?;

            if let PyValue::List(l) = data {
                let mut items = l.to_vec();
                if items.is_empty() {
                    return Err(RuntimeError::value_error("pop from an empty deque"));
                }
                let item = items.remove(0);
                deque.setitem(
                    PyKey::Str(Arc::from("_data")),
                    PyValue::List(Arc::new(PyList::from_values(items))),
                );
                Ok(item)
            } else {
                Err(RuntimeError::internal_error("Invalid deque data"))
            }
        }),
        // namedtuple - factory function for creating tuple subclasses
        PyBuiltinFunction::new("namedtuple", |args| {
            let typename = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let field_names: Vec<String> = match args.get(1) {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Str(s) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Some(PyValue::Tuple(t)) => t
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Str(s) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Some(PyValue::Str(s)) => {
                    // Split by comma or whitespace
                    s.split(|c: char| c == ',' || c.is_whitespace())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.trim().to_string())
                        .collect()
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "sequence of field names",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            // Return a "type" dict that can be used to create instances
            let type_dict = PyDict::new();
            type_dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("namedtuple_type")),
            );
            type_dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from(typename)));
            type_dict.setitem(
                PyKey::Str(Arc::from("_fields")),
                PyValue::Tuple(Arc::new(PyTuple::from_values(
                    field_names.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect(),
                ))),
            );

            Ok(PyValue::Dict(Arc::new(type_dict)))
        }),
        // ChainMap - dict-like class for creating a single view of multiple mappings
        PyBuiltinFunction::new("ChainMap", |args| {
            let maps: Vec<PyValue> = args.iter().cloned().collect();

            let dict = PyDict::new();
            dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("ChainMap")));
            dict.setitem(
                PyKey::Str(Arc::from("maps")),
                PyValue::List(Arc::new(PyList::from_values(if maps.is_empty() {
                    vec![PyValue::Dict(Arc::new(PyDict::new()))]
                } else {
                    maps
                }))),
            );

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // ChainMap.new_child - create new ChainMap with new map followed by all previous maps
        PyBuiltinFunction::new("ChainMap_new_child", |args| {
            let chainmap = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "ChainMap",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let new_map = match args.get(1) {
                Some(PyValue::Dict(d)) => PyValue::Dict(Arc::clone(d)),
                Some(PyValue::None) | None => PyValue::Dict(Arc::new(PyDict::new())),
                Some(v) => return Err(RuntimeError::type_error("dict or None", v.type_name())),
            };

            let maps = chainmap
                .getitem(&PyKey::Str(Arc::from("maps")))
                .map_err(|_| RuntimeError::attribute_error("ChainMap", "maps"))?;

            if let PyValue::List(l) = maps {
                let mut new_maps = vec![new_map];
                new_maps.extend(l.to_vec());

                let new_chainmap = PyDict::new();
                new_chainmap.setitem(
                    PyKey::Str(Arc::from("__class__")),
                    PyValue::Str(Arc::from("ChainMap")),
                );
                new_chainmap.setitem(
                    PyKey::Str(Arc::from("maps")),
                    PyValue::List(Arc::new(PyList::from_values(new_maps))),
                );

                Ok(PyValue::Dict(Arc::new(new_chainmap)))
            } else {
                Err(RuntimeError::internal_error("Invalid ChainMap maps"))
            }
        }),
    ]
}

// ===== itertools module (Task 8.1) =====

/// Get the itertools module as a dict
pub fn itertools_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("itertools")));

    Arc::new(dict)
}

/// Create itertools module builtins
pub fn itertools_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // chain - chain multiple iterables together
        PyBuiltinFunction::new("chain", |args| {
            let mut result = Vec::new();

            for arg in args {
                match arg {
                    PyValue::List(l) => result.extend(l.to_vec()),
                    PyValue::Tuple(t) => result.extend(t.to_vec()),
                    PyValue::Str(s) => {
                        result.extend(s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))));
                    }
                    _ => return Err(RuntimeError::type_error("iterable", arg.type_name())),
                }
            }

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // cycle - cycle through an iterable infinitely (returns first n items)
        // Note: Since we can't return infinite iterators, we take a count parameter
        PyBuiltinFunction::new("cycle", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let count = match args.get(1) {
                Some(PyValue::Int(n)) if *n >= 0 => *n as usize,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("count must be non-negative"))
                }
                None => 100, // Default to 100 items
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                PyValue::Str(s) => {
                    s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                }
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            if items.is_empty() {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            let mut result = Vec::with_capacity(count);
            for i in 0..count {
                result.push(items[i % items.len()].clone());
            }

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // repeat - repeat a value n times
        PyBuiltinFunction::new("repeat", |args| {
            let value = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("value", "nothing")),
            };

            let times = match args.get(1) {
                Some(PyValue::Int(n)) if *n >= 0 => *n as usize,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("times must be non-negative"))
                }
                None => 1, // Default to 1
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let result: Vec<PyValue> = (0..times).map(|_| value.clone()).collect();
            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // count - generate consecutive integers starting from start
        PyBuiltinFunction::new("count", |args| {
            let start = match args.first() {
                Some(PyValue::Int(n)) => *n,
                None => 0,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let step = match args.get(1) {
                Some(PyValue::Int(n)) => *n,
                None => 1,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let count = match args.get(2) {
                Some(PyValue::Int(n)) if *n >= 0 => *n as usize,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("count must be non-negative"))
                }
                None => 100, // Default to 100 items
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let result: Vec<PyValue> =
                (0..count).map(|i| PyValue::Int(start + (i as i64) * step)).collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // combinations - return r-length combinations of elements
        PyBuiltinFunction::new("combinations", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let r = match args.get(1) {
                Some(PyValue::Int(n)) if *n >= 0 => *n as usize,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("r must be non-negative"))
                }
                None => return Err(RuntimeError::type_error("int", "nothing")),
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                PyValue::Str(s) => {
                    s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                }
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            let n = items.len();

            // Handle edge cases
            if r == 0 {
                // C(n, 0) = 1 (empty tuple)
                return Ok(PyValue::List(Arc::new(PyList::from_values(vec![PyValue::Tuple(
                    Arc::new(PyTuple::from_values(vec![])),
                )]))));
            }

            if r > n {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            let mut result = Vec::new();
            let mut indices: Vec<usize> = (0..r).collect();

            // Generate first combination
            let combo: Vec<PyValue> = indices.iter().map(|&i| items[i].clone()).collect();
            result.push(PyValue::Tuple(Arc::new(PyTuple::from_values(combo))));

            // Generate remaining combinations
            loop {
                // Find rightmost index that can be incremented
                let mut i = r;
                while i > 0 {
                    i -= 1;
                    if indices[i] != i + n - r {
                        break;
                    }
                    if i == 0 && indices[i] == n - r {
                        // All combinations generated
                        return Ok(PyValue::List(Arc::new(PyList::from_values(result))));
                    }
                }

                // Increment this index and reset all following indices
                indices[i] += 1;
                for j in (i + 1)..r {
                    indices[j] = indices[j - 1] + 1;
                }

                let combo: Vec<PyValue> = indices.iter().map(|&idx| items[idx].clone()).collect();
                result.push(PyValue::Tuple(Arc::new(PyTuple::from_values(combo))));
            }
        }),
        // permutations - return r-length permutations of elements
        PyBuiltinFunction::new("permutations", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                PyValue::Str(s) => {
                    s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                }
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            let n = items.len();
            let r = match args.get(1) {
                Some(PyValue::Int(r)) if *r >= 0 => (*r as usize).min(n),
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("r must be non-negative"))
                }
                None => n,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            if r > n || n == 0 {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            // Simple implementation for small inputs
            fn generate_permutations(items: &[PyValue], r: usize) -> Vec<Vec<PyValue>> {
                if r == 0 {
                    return vec![vec![]];
                }

                let mut result = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let remaining: Vec<PyValue> = items
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, v)| v.clone())
                        .collect();

                    for mut perm in generate_permutations(&remaining, r - 1) {
                        perm.insert(0, item.clone());
                        result.push(perm);
                    }
                }
                result
            }

            let perms = generate_permutations(&items, r);
            let result: Vec<PyValue> = perms
                .into_iter()
                .map(|p| PyValue::Tuple(Arc::new(PyTuple::from_values(p))))
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // product - cartesian product of input iterables
        PyBuiltinFunction::new("product", |args| {
            if args.is_empty() {
                return Ok(PyValue::List(Arc::new(PyList::from_values(vec![PyValue::Tuple(
                    Arc::new(PyTuple::from_values(vec![])),
                )]))));
            }

            let mut iterables: Vec<Vec<PyValue>> = Vec::new();

            for arg in args {
                let items: Vec<PyValue> = match arg {
                    PyValue::List(l) => l.to_vec(),
                    PyValue::Tuple(t) => t.to_vec(),
                    PyValue::Str(s) => {
                        s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                    }
                    _ => return Err(RuntimeError::type_error("iterable", arg.type_name())),
                };
                iterables.push(items);
            }

            // Check for empty iterables
            if iterables.iter().any(|it| it.is_empty()) {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            // Generate cartesian product
            fn cartesian_product(iterables: &[Vec<PyValue>]) -> Vec<Vec<PyValue>> {
                if iterables.is_empty() {
                    return vec![vec![]];
                }

                let first = &iterables[0];
                let rest = cartesian_product(&iterables[1..]);

                let mut result = Vec::new();
                for item in first {
                    for r in &rest {
                        let mut combo = vec![item.clone()];
                        combo.extend(r.clone());
                        result.push(combo);
                    }
                }
                result
            }

            let products = cartesian_product(&iterables);
            let result: Vec<PyValue> = products
                .into_iter()
                .map(|p| PyValue::Tuple(Arc::new(PyTuple::from_values(p))))
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // groupby - group consecutive elements by key
        // Note: Simplified version that groups by value equality
        PyBuiltinFunction::new("groupby", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            if items.is_empty() {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            // Helper function to compare PyValues
            fn values_equal(a: &PyValue, b: &PyValue) -> bool {
                match (a, b) {
                    (PyValue::None, PyValue::None) => true,
                    (PyValue::Bool(a), PyValue::Bool(b)) => a == b,
                    (PyValue::Int(a), PyValue::Int(b)) => a == b,
                    (PyValue::Float(a), PyValue::Float(b)) => a == b,
                    (PyValue::Str(a), PyValue::Str(b)) => a == b,
                    _ => false,
                }
            }

            let mut result = Vec::new();
            let mut current_key = items[0].clone();
            let mut current_group = vec![items[0].clone()];

            for item in items.into_iter().skip(1) {
                if values_equal(&item, &current_key) {
                    current_group.push(item);
                } else {
                    result.push(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                        current_key,
                        PyValue::List(Arc::new(PyList::from_values(current_group))),
                    ]))));
                    current_key = item.clone();
                    current_group = vec![item];
                }
            }

            // Add last group
            result.push(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                current_key,
                PyValue::List(Arc::new(PyList::from_values(current_group))),
            ]))));

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // islice - slice an iterable
        PyBuiltinFunction::new("islice", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                PyValue::Str(s) => {
                    s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                }
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            // Parse arguments: islice(iterable, stop) or islice(iterable, start, stop[, step])
            let (start, stop, step) = match args.len() {
                2 => {
                    let stop = match &args[1] {
                        PyValue::Int(n) if *n >= 0 => *n as usize,
                        PyValue::Int(_) => {
                            return Err(RuntimeError::value_error("stop must be non-negative"))
                        }
                        PyValue::None => items.len(),
                        v => return Err(RuntimeError::type_error("int or None", v.type_name())),
                    };
                    (0, stop, 1)
                }
                3 | 4 => {
                    let start = match &args[1] {
                        PyValue::Int(n) if *n >= 0 => *n as usize,
                        PyValue::Int(_) => {
                            return Err(RuntimeError::value_error("start must be non-negative"))
                        }
                        PyValue::None => 0,
                        v => return Err(RuntimeError::type_error("int or None", v.type_name())),
                    };
                    let stop = match &args[2] {
                        PyValue::Int(n) if *n >= 0 => *n as usize,
                        PyValue::Int(_) => {
                            return Err(RuntimeError::value_error("stop must be non-negative"))
                        }
                        PyValue::None => items.len(),
                        v => return Err(RuntimeError::type_error("int or None", v.type_name())),
                    };
                    let step = if args.len() == 4 {
                        match &args[3] {
                            PyValue::Int(n) if *n > 0 => *n as usize,
                            PyValue::Int(_) => {
                                return Err(RuntimeError::value_error("step must be positive"))
                            }
                            PyValue::None => 1,
                            v => {
                                return Err(RuntimeError::type_error("int or None", v.type_name()))
                            }
                        }
                    } else {
                        1
                    };
                    (start, stop, step)
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "2-4 arguments",
                        format!("{} arguments", args.len()),
                    ))
                }
            };

            let result: Vec<PyValue> = items
                .into_iter()
                .skip(start)
                .take(stop.saturating_sub(start))
                .step_by(step)
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // takewhile - take items while predicate is true
        // Note: Simplified version that takes while items are truthy
        PyBuiltinFunction::new("takewhile", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            // Take while items are truthy (simplified - no predicate function)
            let result: Vec<PyValue> = items
                .into_iter()
                .take_while(|item| match item {
                    PyValue::Bool(b) => *b,
                    PyValue::Int(n) => *n != 0,
                    PyValue::Float(f) => *f != 0.0,
                    PyValue::Str(s) => !s.is_empty(),
                    PyValue::List(l) => !l.is_empty(),
                    PyValue::Dict(d) => !d.is_empty(),
                    PyValue::None => false,
                    _ => true,
                })
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // dropwhile - drop items while predicate is true
        // Note: Simplified version that drops while items are truthy
        PyBuiltinFunction::new("dropwhile", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            // Drop while items are truthy (simplified - no predicate function)
            let result: Vec<PyValue> = items
                .into_iter()
                .skip_while(|item| match item {
                    PyValue::Bool(b) => *b,
                    PyValue::Int(n) => *n != 0,
                    PyValue::Float(f) => *f != 0.0,
                    PyValue::Str(s) => !s.is_empty(),
                    PyValue::List(l) => !l.is_empty(),
                    PyValue::Dict(d) => !d.is_empty(),
                    PyValue::None => false,
                    _ => true,
                })
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // accumulate - make an iterator that returns accumulated sums
        PyBuiltinFunction::new("accumulate", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            if items.is_empty() {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            let mut result = Vec::with_capacity(items.len());
            let mut acc = items[0].clone();
            result.push(acc.clone());

            for item in items.into_iter().skip(1) {
                // Try to add (only works for ints and floats)
                acc = match (&acc, &item) {
                    (PyValue::Int(a), PyValue::Int(b)) => PyValue::Int(a + b),
                    (PyValue::Float(a), PyValue::Float(b)) => PyValue::Float(a + b),
                    (PyValue::Int(a), PyValue::Float(b)) => PyValue::Float(*a as f64 + b),
                    (PyValue::Float(a), PyValue::Int(b)) => PyValue::Float(a + *b as f64),
                    (PyValue::Str(a), PyValue::Str(b)) => {
                        PyValue::Str(Arc::from(format!("{}{}", a, b)))
                    }
                    _ => item.clone(), // Can't accumulate, just use the item
                };
                result.push(acc.clone());
            }

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // compress - filter elements based on selectors
        PyBuiltinFunction::new("compress", |args| {
            let data = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("data", "nothing")),
            };

            let selectors = match args.get(1) {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("selectors", "nothing")),
            };

            let data_items: Vec<PyValue> = match data {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                PyValue::Str(s) => {
                    s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                }
                _ => return Err(RuntimeError::type_error("iterable", data.type_name())),
            };

            let selector_items: Vec<PyValue> = match selectors {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", selectors.type_name())),
            };

            let result: Vec<PyValue> = data_items
                .into_iter()
                .zip(selector_items.into_iter())
                .filter_map(|(d, s)| {
                    let is_truthy = match s {
                        PyValue::Bool(b) => b,
                        PyValue::Int(n) => n != 0,
                        PyValue::Float(f) => f != 0.0,
                        PyValue::Str(s) => !s.is_empty(),
                        PyValue::None => false,
                        _ => true,
                    };
                    if is_truthy {
                        Some(d)
                    } else {
                        None
                    }
                })
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // filterfalse - filter elements where predicate is false
        // Note: Simplified version that filters falsy items
        PyBuiltinFunction::new("filterfalse", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            // Filter items that are falsy
            let result: Vec<PyValue> = items
                .into_iter()
                .filter(|item| match item {
                    PyValue::Bool(b) => !*b,
                    PyValue::Int(n) => *n == 0,
                    PyValue::Float(f) => *f == 0.0,
                    PyValue::Str(s) => s.is_empty(),
                    PyValue::List(l) => l.is_empty(),
                    PyValue::Dict(d) => d.is_empty(),
                    PyValue::None => true,
                    _ => false,
                })
                .collect();

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // starmap - apply function to arguments from iterable
        // Note: Simplified version that just unpacks tuples
        PyBuiltinFunction::new("starmap", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            // Just return the items as-is (simplified - no function application)
            Ok(PyValue::List(Arc::new(PyList::from_values(items))))
        }),
        // zip_longest - zip iterables, filling missing values with fillvalue
        PyBuiltinFunction::new("zip_longest", |args| {
            if args.is_empty() {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            // Find fillvalue (last arg if it's not an iterable)
            let fillvalue = PyValue::None;

            let mut iterables: Vec<Vec<PyValue>> = Vec::new();

            for arg in args {
                let items: Vec<PyValue> = match arg {
                    PyValue::List(l) => l.to_vec(),
                    PyValue::Tuple(t) => t.to_vec(),
                    PyValue::Str(s) => {
                        s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                    }
                    _ => return Err(RuntimeError::type_error("iterable", arg.type_name())),
                };
                iterables.push(items);
            }

            if iterables.is_empty() {
                return Ok(PyValue::List(Arc::new(PyList::new())));
            }

            let max_len = iterables.iter().map(|it| it.len()).max().unwrap_or(0);

            let mut result = Vec::with_capacity(max_len);
            for i in 0..max_len {
                let tuple_items: Vec<PyValue> = iterables
                    .iter()
                    .map(|it| it.get(i).cloned().unwrap_or_else(|| fillvalue.clone()))
                    .collect();
                result.push(PyValue::Tuple(Arc::new(PyTuple::from_values(tuple_items))));
            }

            Ok(PyValue::List(Arc::new(PyList::from_values(result))))
        }),
        // tee - return n independent iterators from a single iterable
        PyBuiltinFunction::new("tee", |args| {
            let iterable = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let n = match args.get(1) {
                Some(PyValue::Int(n)) if *n >= 0 => *n as usize,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("n must be non-negative"))
                }
                None => 2,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                PyValue::Str(s) => {
                    s.chars().map(|c| PyValue::Str(Arc::from(c.to_string()))).collect()
                }
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            // Return n copies of the list
            let result: Vec<PyValue> = (0..n)
                .map(|_| PyValue::List(Arc::new(PyList::from_values(items.clone()))))
                .collect();

            Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(result))))
        }),
    ]
}

// ===== functools module (Task 8.2) =====

/// Get the functools module as a dict
pub fn functools_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("functools")));

    // WRAPPER_ASSIGNMENTS and WRAPPER_UPDATES constants
    dict.setitem(
        PyKey::Str(Arc::from("WRAPPER_ASSIGNMENTS")),
        PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
            PyValue::Str(Arc::from("__module__")),
            PyValue::Str(Arc::from("__name__")),
            PyValue::Str(Arc::from("__qualname__")),
            PyValue::Str(Arc::from("__annotations__")),
            PyValue::Str(Arc::from("__doc__")),
        ]))),
    );

    dict.setitem(
        PyKey::Str(Arc::from("WRAPPER_UPDATES")),
        PyValue::Tuple(Arc::new(PyTuple::from_values(vec![PyValue::Str(Arc::from("__dict__"))]))),
    );

    Arc::new(dict)
}

/// Create functools module builtins
pub fn functools_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // partial - create a partial function application
        PyBuiltinFunction::new("partial", |args| {
            if args.is_empty() {
                return Err(RuntimeError::type_error("callable", "nothing"));
            }

            let func = args[0].clone();
            let partial_args: Vec<PyValue> = args[1..].to_vec();

            let dict = PyDict::new();
            dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("partial")));
            dict.setitem(PyKey::Str(Arc::from("func")), func);
            dict.setitem(
                PyKey::Str(Arc::from("args")),
                PyValue::Tuple(Arc::new(PyTuple::from_values(partial_args))),
            );
            dict.setitem(PyKey::Str(Arc::from("keywords")), PyValue::Dict(Arc::new(PyDict::new())));

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // reduce - apply function cumulatively to items
        PyBuiltinFunction::new("reduce", |args| {
            // Note: This is a simplified version that works with specific operations
            let iterable = match args.get(1) {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("iterable", "nothing")),
            };

            let items: Vec<PyValue> = match iterable {
                PyValue::List(l) => l.to_vec(),
                PyValue::Tuple(t) => t.to_vec(),
                _ => return Err(RuntimeError::type_error("iterable", iterable.type_name())),
            };

            // Get initial value if provided
            let (start_idx, mut accumulator) = match args.get(2) {
                Some(initial) => (0, initial.clone()),
                None => {
                    if items.is_empty() {
                        return Err(RuntimeError::type_error(
                            "reduce() of empty sequence with no initial value",
                            "empty",
                        ));
                    }
                    (1, items[0].clone())
                }
            };

            // Apply reduction (simplified - just accumulates for numeric types)
            for item in items.into_iter().skip(start_idx) {
                accumulator = match (&accumulator, &item) {
                    (PyValue::Int(a), PyValue::Int(b)) => PyValue::Int(a + b),
                    (PyValue::Float(a), PyValue::Float(b)) => PyValue::Float(a + b),
                    (PyValue::Int(a), PyValue::Float(b)) => PyValue::Float(*a as f64 + b),
                    (PyValue::Float(a), PyValue::Int(b)) => PyValue::Float(a + *b as f64),
                    (PyValue::Str(a), PyValue::Str(b)) => {
                        PyValue::Str(Arc::from(format!("{}{}", a, b)))
                    }
                    _ => item, // Can't reduce, just use the item
                };
            }

            Ok(accumulator)
        }),
        // lru_cache - decorator that caches function results
        // Note: Returns a dict representing the cached function
        PyBuiltinFunction::new("lru_cache", |args| {
            let maxsize = match args.first() {
                Some(PyValue::Int(n)) if *n >= 0 => Some(*n as usize),
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("maxsize must be non-negative"))
                }
                Some(PyValue::None) | None => None, // Unbounded cache
                Some(v) => return Err(RuntimeError::type_error("int or None", v.type_name())),
            };

            let typed = match args.get(1) {
                Some(PyValue::Bool(b)) => *b,
                None => false,
                Some(v) => return Err(RuntimeError::type_error("bool", v.type_name())),
            };

            let dict = PyDict::new();
            dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("lru_cache")));
            dict.setitem(
                PyKey::Str(Arc::from("maxsize")),
                match maxsize {
                    Some(n) => PyValue::Int(n as i64),
                    None => PyValue::None,
                },
            );
            dict.setitem(PyKey::Str(Arc::from("typed")), PyValue::Bool(typed));
            dict.setitem(PyKey::Str(Arc::from("_cache")), PyValue::Dict(Arc::new(PyDict::new())));
            dict.setitem(PyKey::Str(Arc::from("_hits")), PyValue::Int(0));
            dict.setitem(PyKey::Str(Arc::from("_misses")), PyValue::Int(0));

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // cache_info - get cache statistics
        PyBuiltinFunction::new("lru_cache_info", |args| {
            let cache = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "lru_cache",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let hits = cache.getitem(&PyKey::Str(Arc::from("_hits"))).unwrap_or(PyValue::Int(0));
            let misses =
                cache.getitem(&PyKey::Str(Arc::from("_misses"))).unwrap_or(PyValue::Int(0));
            let maxsize = cache.getitem(&PyKey::Str(Arc::from("maxsize"))).unwrap_or(PyValue::None);
            let currsize = match cache.getitem(&PyKey::Str(Arc::from("_cache"))) {
                Ok(PyValue::Dict(d)) => PyValue::Int(d.len() as i64),
                _ => PyValue::Int(0),
            };

            // Return named tuple-like dict
            let info = PyDict::new();
            info.setitem(PyKey::Str(Arc::from("hits")), hits);
            info.setitem(PyKey::Str(Arc::from("misses")), misses);
            info.setitem(PyKey::Str(Arc::from("maxsize")), maxsize);
            info.setitem(PyKey::Str(Arc::from("currsize")), currsize);

            Ok(PyValue::Dict(Arc::new(info)))
        }),
        // cache_clear - clear the cache
        PyBuiltinFunction::new("lru_cache_clear", |args| {
            let cache = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "lru_cache",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            cache.setitem(PyKey::Str(Arc::from("_cache")), PyValue::Dict(Arc::new(PyDict::new())));
            cache.setitem(PyKey::Str(Arc::from("_hits")), PyValue::Int(0));
            cache.setitem(PyKey::Str(Arc::from("_misses")), PyValue::Int(0));

            Ok(PyValue::None)
        }),
        // wraps - decorator to copy function metadata
        // Note: Returns a dict representing the wrapper
        PyBuiltinFunction::new("wraps", |args| {
            let wrapped = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("callable", "nothing")),
            };

            let dict = PyDict::new();
            dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("wraps")));
            dict.setitem(PyKey::Str(Arc::from("__wrapped__")), wrapped);

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // update_wrapper - update wrapper function to look like wrapped function
        PyBuiltinFunction::new("update_wrapper", |args| {
            let wrapper = match args.first() {
                Some(PyValue::Dict(d)) => d.clone(),
                Some(v) => return Err(RuntimeError::type_error("dict", v.type_name())),
                None => return Err(RuntimeError::type_error("wrapper", "nothing")),
            };

            let wrapped = match args.get(1) {
                Some(PyValue::Dict(d)) => d.clone(),
                Some(v) => return Err(RuntimeError::type_error("dict", v.type_name())),
                None => return Err(RuntimeError::type_error("wrapped", "nothing")),
            };

            // Copy attributes from wrapped to wrapper
            let attrs_to_copy = [
                "__module__",
                "__name__",
                "__qualname__",
                "__doc__",
                "__annotations__",
            ];
            for attr in attrs_to_copy {
                if let Ok(value) = wrapped.getitem(&PyKey::Str(Arc::from(attr))) {
                    wrapper.setitem(PyKey::Str(Arc::from(attr)), value);
                }
            }

            // Set __wrapped__
            wrapper.setitem(PyKey::Str(Arc::from("__wrapped__")), PyValue::Dict(wrapped));

            Ok(PyValue::Dict(wrapper))
        }),
        // cached_property - decorator that converts a method into a cached property
        PyBuiltinFunction::new("cached_property", |args| {
            let func = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("callable", "nothing")),
            };

            let dict = PyDict::new();
            dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("cached_property")),
            );
            dict.setitem(PyKey::Str(Arc::from("func")), func);
            dict.setitem(PyKey::Str(Arc::from("attrname")), PyValue::None);
            dict.setitem(PyKey::Str(Arc::from("__doc__")), PyValue::None);

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // cmp_to_key - convert a cmp function to a key function
        PyBuiltinFunction::new("cmp_to_key", |args| {
            let cmp_func = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("callable", "nothing")),
            };

            let dict = PyDict::new();
            dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("cmp_to_key")));
            dict.setitem(PyKey::Str(Arc::from("cmp")), cmp_func);

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
        // total_ordering - class decorator that fills in missing ordering methods
        // Note: Returns a marker dict
        PyBuiltinFunction::new("total_ordering", |args| {
            let cls = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("class", "nothing")),
            };

            // Just return the class as-is (simplified)
            Ok(cls)
        }),
        // singledispatch - single-dispatch generic function decorator
        PyBuiltinFunction::new("singledispatch", |args| {
            let func = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("callable", "nothing")),
            };

            let dict = PyDict::new();
            dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("singledispatch")),
            );
            dict.setitem(PyKey::Str(Arc::from("func")), func);
            dict.setitem(PyKey::Str(Arc::from("registry")), PyValue::Dict(Arc::new(PyDict::new())));

            Ok(PyValue::Dict(Arc::new(dict)))
        }),
    ]
}

// ===== json module expansion (Task 9.1) =====

/// Get the json module as a dict
pub fn json_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("json")));

    Arc::new(dict)
}

/// JSON encoding options
#[derive(Clone)]
struct JsonEncodeOptions {
    indent: Option<usize>,
    separators: Option<(String, String)>,
    sort_keys: bool,
    ensure_ascii: bool,
}

impl Default for JsonEncodeOptions {
    fn default() -> Self {
        Self {
            indent: None,
            separators: None,
            sort_keys: false,
            ensure_ascii: false,
        }
    }
}

/// Convert PyValue to JSON string with options
fn value_to_json_with_options(
    value: &PyValue,
    options: &JsonEncodeOptions,
    depth: usize,
) -> RuntimeResult<String> {
    let (item_sep, key_sep) = options.separators.clone().unwrap_or_else(|| {
        if options.indent.is_some() {
            (", ".to_string(), ": ".to_string())
        } else {
            (",".to_string(), ":".to_string())
        }
    });

    let indent_str = options.indent.map(|n| " ".repeat(n));
    let newline = if options.indent.is_some() { "\n" } else { "" };

    match value {
        PyValue::None => Ok("null".to_string()),
        PyValue::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        PyValue::Int(i) => Ok(i.to_string()),
        PyValue::Float(f) => {
            if f.is_nan() || f.is_infinite() {
                return Err(RuntimeError::value_error(
                    "Out of range float values are not JSON compliant",
                ));
            }
            Ok(f.to_string())
        }
        PyValue::Str(s) => {
            if options.ensure_ascii {
                Ok(format!("\"{}\"", escape_json_string_ascii(s)))
            } else {
                Ok(format!("\"{}\"", escape_json_string(s)))
            }
        }
        PyValue::List(list) => {
            let items = list.to_vec();
            if items.is_empty() {
                return Ok("[]".to_string());
            }

            let inner: RuntimeResult<Vec<String>> = items
                .iter()
                .map(|v| value_to_json_with_options(v, options, depth + 1))
                .collect();
            let inner = inner?;

            if let Some(ref indent) = indent_str {
                let inner_indent = indent.repeat(depth + 1);
                let outer_indent = indent.repeat(depth);
                Ok(format!(
                    "[{}{}{}{}\n{}]",
                    newline,
                    inner_indent,
                    inner.join(&format!("{}{}{}", item_sep.trim_end(), newline, inner_indent)),
                    newline,
                    outer_indent
                ))
            } else {
                Ok(format!("[{}]", inner.join(&item_sep)))
            }
        }
        PyValue::Set(set) => {
            // Sets are serialized as arrays (JSON doesn't have sets)
            let items = set.to_vec();
            if items.is_empty() {
                return Ok("[]".to_string());
            }

            let inner: RuntimeResult<Vec<String>> = items
                .iter()
                .map(|v| value_to_json_with_options(v, options, depth + 1))
                .collect();
            let inner = inner?;

            if let Some(ref indent) = indent_str {
                let inner_indent = indent.repeat(depth + 1);
                let outer_indent = indent.repeat(depth);
                Ok(format!(
                    "[{}{}{}{}\n{}]",
                    newline,
                    inner_indent,
                    inner.join(&format!("{}{}{}", item_sep.trim_end(), newline, inner_indent)),
                    newline,
                    outer_indent
                ))
            } else {
                Ok(format!("[{}]", inner.join(&item_sep)))
            }
        }
        PyValue::Dict(dict) => {
            let items = dict.items();
            if items.is_empty() {
                return Ok("{}".to_string());
            }

            // Optionally sort keys
            let mut pairs: Vec<_> = items.iter().collect();
            if options.sort_keys {
                pairs.sort_by(|(k1, _), (k2, _)| {
                    let s1 = match k1 {
                        crate::pydict::PyKey::Str(s) => s.to_string(),
                        crate::pydict::PyKey::Int(i) => i.to_string(),
                        _ => String::new(),
                    };
                    let s2 = match k2 {
                        crate::pydict::PyKey::Str(s) => s.to_string(),
                        crate::pydict::PyKey::Int(i) => i.to_string(),
                        _ => String::new(),
                    };
                    s1.cmp(&s2)
                });
            }

            let inner: RuntimeResult<Vec<String>> = pairs
                .iter()
                .map(|(k, v)| {
                    let key_str = match k {
                        crate::pydict::PyKey::Str(s) => {
                            if options.ensure_ascii {
                                format!("\"{}\"", escape_json_string_ascii(s))
                            } else {
                                format!("\"{}\"", escape_json_string(s))
                            }
                        }
                        crate::pydict::PyKey::Int(i) => format!("\"{}\"", i),
                        _ => return Err(RuntimeError::type_error("str key", "non-str key")),
                    };
                    let val_str = value_to_json_with_options(v, options, depth + 1)?;
                    Ok(format!("{}{}{}", key_str, key_sep, val_str))
                })
                .collect();
            let inner = inner?;

            if let Some(ref indent) = indent_str {
                let inner_indent = indent.repeat(depth + 1);
                let outer_indent = indent.repeat(depth);
                Ok(format!(
                    "{{{}{}{}{}\n{}}}",
                    newline,
                    inner_indent,
                    inner.join(&format!("{}{}{}", item_sep.trim_end(), newline, inner_indent)),
                    newline,
                    outer_indent
                ))
            } else {
                Ok(format!("{{{}}}", inner.join(&item_sep)))
            }
        }
        PyValue::Tuple(tuple) => {
            // Tuples are serialized as arrays
            let items = tuple.to_vec();
            if items.is_empty() {
                return Ok("[]".to_string());
            }

            let inner: RuntimeResult<Vec<String>> = items
                .iter()
                .map(|v| value_to_json_with_options(v, options, depth + 1))
                .collect();
            let inner = inner?;

            if let Some(ref indent) = indent_str {
                let inner_indent = indent.repeat(depth + 1);
                let outer_indent = indent.repeat(depth);
                Ok(format!(
                    "[{}{}{}{}\n{}]",
                    newline,
                    inner_indent,
                    inner.join(&format!("{}{}{}", item_sep.trim_end(), newline, inner_indent)),
                    newline,
                    outer_indent
                ))
            } else {
                Ok(format!("[{}]", inner.join(&item_sep)))
            }
        }
        PyValue::Exception(e) => {
            // Serialize exception as a dict with type and message
            let type_str = if options.ensure_ascii {
                escape_json_string_ascii(&e.exc_type)
            } else {
                escape_json_string(&e.exc_type)
            };
            let msg_str = if options.ensure_ascii {
                escape_json_string_ascii(&e.message)
            } else {
                escape_json_string(&e.message)
            };
            Ok(format!(
                "{{\"type\"{}\"{}\"{}\"message\"{}\"{}\"}}",
                key_sep, type_str, item_sep, key_sep, msg_str
            ))
        }
        PyValue::Type(t) => {
            // Serialize type as a string representation
            let type_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<class '{}'>", t.name))
            } else {
                escape_json_string(&format!("<class '{}'>", t.name))
            };
            Ok(format!("\"{}\"", type_str))
        }
        PyValue::Instance(inst) => {
            // Serialize instance as a string representation
            let inst_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<{} object>", inst.class.name))
            } else {
                escape_json_string(&format!("<{} object>", inst.class.name))
            };
            Ok(format!("\"{}\"", inst_str))
        }
        PyValue::BoundMethod(_) => {
            // Serialize bound method as a string representation
            let method_str = if options.ensure_ascii {
                escape_json_string_ascii("<bound method>")
            } else {
                escape_json_string("<bound method>")
            };
            Ok(format!("\"{}\"", method_str))
        }
        PyValue::Generator(gen) => {
            // Serialize generator as a string representation
            let gen_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<generator object {}>", gen.name))
            } else {
                escape_json_string(&format!("<generator object {}>", gen.name))
            };
            Ok(format!("\"{}\"", gen_str))
        }
        PyValue::Coroutine(coro) => {
            // Serialize coroutine as a string representation
            let coro_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<coroutine object {}>", coro.name))
            } else {
                escape_json_string(&format!("<coroutine object {}>", coro.name))
            };
            Ok(format!("\"{}\"", coro_str))
        }
        PyValue::Builtin(b) => {
            let builtin_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<built-in function {}>", b.name))
            } else {
                escape_json_string(&format!("<built-in function {}>", b.name))
            };
            Ok(format!("\"{}\"", builtin_str))
        }
        PyValue::Function(f) => {
            let func_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<function {}>", f.name))
            } else {
                escape_json_string(&format!("<function {}>", f.name))
            };
            Ok(format!("\"{}\"", func_str))
        }
        PyValue::Iterator(_) => {
            let iter_str = if options.ensure_ascii {
                escape_json_string_ascii("<iterator>")
            } else {
                escape_json_string("<iterator>")
            };
            Ok(format!("\"{}\"", iter_str))
        }
        PyValue::Module(m) => {
            let mod_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<module '{}'>", m.name))
            } else {
                escape_json_string(&format!("<module '{}'>", m.name))
            };
            Ok(format!("\"{}\"", mod_str))
        }
        PyValue::Code(c) => {
            let code_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<code object {}>", c.name))
            } else {
                escape_json_string(&format!("<code object {}>", c.name))
            };
            Ok(format!("\"{}\"", code_str))
        }
        PyValue::Cell(cell) => {
            // Serialize the cell's contents
            value_to_json_with_options(&cell.get(), options, depth)
        }
        PyValue::Super(s) => {
            let super_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!("<super: <class '{}'>>", s.type_.name))
            } else {
                escape_json_string(&format!("<super: <class '{}'>>", s.type_.name))
            };
            Ok(format!("\"{}\"", super_str))
        }
        PyValue::Property(p) => {
            let prop_str = if options.ensure_ascii {
                escape_json_string_ascii(&format!(
                    "<property: {}>",
                    p.get_doc().unwrap_or("no doc")
                ))
            } else {
                escape_json_string(&format!("<property: {}>", p.get_doc().unwrap_or("no doc")))
            };
            Ok(format!("\"{}\"", prop_str))
        }
        PyValue::StaticMethod(_) => {
            let sm_str = if options.ensure_ascii {
                escape_json_string_ascii("<staticmethod>")
            } else {
                escape_json_string("<staticmethod>")
            };
            Ok(format!("\"{}\"", sm_str))
        }
        PyValue::ClassMethod(_) => {
            let cm_str = if options.ensure_ascii {
                escape_json_string_ascii("<classmethod>")
            } else {
                escape_json_string("<classmethod>")
            };
            Ok(format!("\"{}\"", cm_str))
        }
    }
}

/// Escape JSON string with ASCII-only output
fn escape_json_string_ascii(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() || !c.is_ascii() => {
                if c as u32 <= 0xFFFF {
                    result.push_str(&format!("\\u{:04x}", c as u32));
                } else {
                    // Surrogate pair for characters outside BMP
                    let code = c as u32 - 0x10000;
                    let high = 0xD800 + (code >> 10);
                    let low = 0xDC00 + (code & 0x3FF);
                    result.push_str(&format!("\\u{:04x}\\u{:04x}", high, low));
                }
            }
            c => result.push(c),
        }
    }
    result
}

/// Parse JSON string to PyValue with proper nested structure handling
fn json_to_value_nested(s: &str) -> RuntimeResult<PyValue> {
    let s_trimmed = s.trim();

    if s_trimmed.is_empty() {
        return Err(RuntimeError::json_decode_error("Expecting value", 1, 1, 0));
    }

    // Use a parser that tracks position
    let mut parser = JsonParser::new(s);
    parser.parse_value()
}

/// JSON parser with position tracking for better error messages
struct JsonParser<'a> {
    input: &'a str,
    pos: usize,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            chars: input.char_indices().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&(_, c)) = self.chars.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        if let Some((i, c)) = self.chars.next() {
            self.pos = i;
            Some((i, c))
        } else {
            None
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    fn peek_pos(&mut self) -> usize {
        self.chars.peek().map(|&(i, _)| i).unwrap_or(self.input.len())
    }

    fn error(&self, message: &str) -> RuntimeError {
        RuntimeError::json_decode_error_at_pos(message, self.input, self.pos)
    }

    fn error_at(&self, message: &str, pos: usize) -> RuntimeError {
        RuntimeError::json_decode_error_at_pos(message, self.input, pos)
    }

    fn parse_value(&mut self) -> RuntimeResult<PyValue> {
        self.skip_whitespace();

        let start_pos = self.peek_pos();

        match self.peek() {
            None => Err(self.error_at("Expecting value", start_pos)),
            Some('n') => self.parse_null(),
            Some('t') => self.parse_true(),
            Some('f') => self.parse_false(),
            Some('"') => self.parse_string(),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(self.error_at(&format!("Unexpected character: {}", c), start_pos)),
        }
    }

    fn parse_null(&mut self) -> RuntimeResult<PyValue> {
        let start = self.peek_pos();
        if self.consume_literal("null") {
            Ok(PyValue::None)
        } else {
            Err(self.error_at("Expected 'null'", start))
        }
    }

    fn parse_true(&mut self) -> RuntimeResult<PyValue> {
        let start = self.peek_pos();
        if self.consume_literal("true") {
            Ok(PyValue::Bool(true))
        } else {
            Err(self.error_at("Expected 'true'", start))
        }
    }

    fn parse_false(&mut self) -> RuntimeResult<PyValue> {
        let start = self.peek_pos();
        if self.consume_literal("false") {
            Ok(PyValue::Bool(false))
        } else {
            Err(self.error_at("Expected 'false'", start))
        }
    }

    fn consume_literal(&mut self, literal: &str) -> bool {
        let start_pos = self.peek_pos();
        for expected in literal.chars() {
            match self.advance() {
                Some((_, c)) if c == expected => continue,
                _ => {
                    // Reset position tracking (can't actually reset iterator)
                    self.pos = start_pos;
                    return false;
                }
            }
        }
        true
    }

    fn parse_string(&mut self) -> RuntimeResult<PyValue> {
        let start_pos = self.peek_pos();

        // Consume opening quote
        match self.advance() {
            Some((_, '"')) => {}
            _ => return Err(self.error_at("Expected '\"'", start_pos)),
        }

        let mut result = String::new();

        loop {
            match self.advance() {
                None => return Err(self.error("Unterminated string starting")),
                Some((_, '"')) => return Ok(PyValue::Str(Arc::from(result))),
                Some((pos, '\\')) => {
                    // Handle escape sequence
                    match self.advance() {
                        None => return Err(self.error("Unterminated string")),
                        Some((_, '"')) => result.push('"'),
                        Some((_, '\\')) => result.push('\\'),
                        Some((_, '/')) => result.push('/'),
                        Some((_, 'b')) => result.push('\x08'),
                        Some((_, 'f')) => result.push('\x0C'),
                        Some((_, 'n')) => result.push('\n'),
                        Some((_, 'r')) => result.push('\r'),
                        Some((_, 't')) => result.push('\t'),
                        Some((_, 'u')) => {
                            let code = self.parse_unicode_escape()?;
                            // Handle surrogate pairs
                            if (0xD800..=0xDBFF).contains(&code) {
                                // High surrogate - expect \uXXXX low surrogate
                                match (self.advance(), self.advance()) {
                                    (Some((_, '\\')), Some((_, 'u'))) => {
                                        let low = self.parse_unicode_escape()?;
                                        if !(0xDC00..=0xDFFF).contains(&low) {
                                            return Err(self.error("Invalid surrogate pair"));
                                        }
                                        let code_point = 0x10000
                                            + ((code as u32 - 0xD800) << 10)
                                            + (low as u32 - 0xDC00);
                                        if let Some(c) = char::from_u32(code_point) {
                                            result.push(c);
                                        }
                                    }
                                    _ => return Err(self.error("Invalid surrogate pair")),
                                }
                            } else if let Some(c) = char::from_u32(code as u32) {
                                result.push(c);
                            } else {
                                return Err(self.error_at("Invalid unicode code point", pos));
                            }
                        }
                        Some((esc_pos, c)) => {
                            return Err(self.error_at(
                                &format!("Invalid escape character: \\{}", c),
                                esc_pos,
                            ))
                        }
                    }
                }
                Some((pos, c)) if c.is_control() => {
                    return Err(
                        self.error_at(&format!("Invalid control character in string"), pos)
                    )
                }
                Some((_, c)) => result.push(c),
            }
        }
    }

    fn parse_unicode_escape(&mut self) -> RuntimeResult<u16> {
        let mut hex = String::with_capacity(4);
        for _ in 0..4 {
            match self.advance() {
                Some((_, c)) if c.is_ascii_hexdigit() => hex.push(c),
                Some((pos, _)) => {
                    return Err(self.error_at("Invalid unicode escape sequence", pos))
                }
                None => return Err(self.error("Incomplete unicode escape sequence")),
            }
        }
        u16::from_str_radix(&hex, 16)
            .map_err(|_| self.error("Invalid unicode escape sequence"))
    }

    fn parse_number(&mut self) -> RuntimeResult<PyValue> {
        let start_pos = self.peek_pos();
        let mut num_str = String::new();
        let mut is_float = false;

        // Optional minus sign
        if self.peek() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        // Integer part
        match self.peek() {
            Some('0') => {
                num_str.push('0');
                self.advance();
            }
            Some(c) if c.is_ascii_digit() && c != '0' => {
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            _ => return Err(self.error_at("Invalid number", start_pos)),
        }

        // Fractional part
        if self.peek() == Some('.') {
            is_float = true;
            num_str.push('.');
            self.advance();

            let mut has_digit = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    self.advance();
                    has_digit = true;
                } else {
                    break;
                }
            }
            if !has_digit {
                return Err(self.error("Invalid number: expected digit after decimal point"));
            }
        }

        // Exponent part
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                is_float = true;
                num_str.push(c);
                self.advance();

                // Optional sign
                if let Some(c) = self.peek() {
                    if c == '+' || c == '-' {
                        num_str.push(c);
                        self.advance();
                    }
                }

                let mut has_digit = false;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        self.advance();
                        has_digit = true;
                    } else {
                        break;
                    }
                }
                if !has_digit {
                    return Err(self.error("Invalid number: expected digit in exponent"));
                }
            }
        }

        if is_float {
            num_str
                .parse::<f64>()
                .map(PyValue::Float)
                .map_err(|_| self.error_at("Invalid number", start_pos))
        } else {
            num_str
                .parse::<i64>()
                .map(PyValue::Int)
                .map_err(|_| self.error_at("Invalid number", start_pos))
        }
    }

    fn parse_array(&mut self) -> RuntimeResult<PyValue> {
        let start_pos = self.peek_pos();

        // Consume opening bracket
        match self.advance() {
            Some((_, '[')) => {}
            _ => return Err(self.error_at("Expected '['", start_pos)),
        }

        self.skip_whitespace();

        // Empty array
        if self.peek() == Some(']') {
            self.advance();
            return Ok(PyValue::List(Arc::new(crate::PyList::new())));
        }

        let mut items = Vec::new();

        loop {
            // Parse value
            let value = self.parse_value()?;
            items.push(value);

            self.skip_whitespace();

            match self.peek() {
                Some(']') => {
                    self.advance();
                    return Ok(PyValue::List(Arc::new(crate::PyList::from_values(items))));
                }
                Some(',') => {
                    self.advance();
                    self.skip_whitespace();
                    // Check for trailing comma (invalid in JSON)
                    if self.peek() == Some(']') {
                        return Err(self.error("Trailing comma in array"));
                    }
                }
                Some(_) => return Err(self.error("Expected ',' or ']' in array")),
                None => return Err(self.error("Unterminated array")),
            }
        }
    }

    fn parse_object(&mut self) -> RuntimeResult<PyValue> {
        let start_pos = self.peek_pos();

        // Consume opening brace
        match self.advance() {
            Some((_, '{')) => {}
            _ => return Err(self.error_at("Expected '{'", start_pos)),
        }

        self.skip_whitespace();

        // Empty object
        if self.peek() == Some('}') {
            self.advance();
            return Ok(PyValue::Dict(Arc::new(PyDict::new())));
        }

        let dict = PyDict::new();

        loop {
            self.skip_whitespace();

            // Parse key (must be a string)
            let key_pos = self.peek_pos();
            if self.peek() != Some('"') {
                return Err(self.error_at("Expected string key in object", key_pos));
            }
            let key = match self.parse_string()? {
                PyValue::Str(s) => s,
                _ => return Err(self.error_at("Object keys must be strings", key_pos)),
            };

            self.skip_whitespace();

            // Expect colon
            match self.advance() {
                Some((_, ':')) => {}
                _ => return Err(self.error("Expected ':' after object key")),
            }

            self.skip_whitespace();

            // Parse value
            let value = self.parse_value()?;
            dict.setitem(PyKey::Str(key), value);

            self.skip_whitespace();

            match self.peek() {
                Some('}') => {
                    self.advance();
                    return Ok(PyValue::Dict(Arc::new(dict)));
                }
                Some(',') => {
                    self.advance();
                    self.skip_whitespace();
                    // Check for trailing comma (invalid in JSON)
                    if self.peek() == Some('}') {
                        return Err(self.error("Trailing comma in object"));
                    }
                }
                Some(_) => return Err(self.error("Expected ',' or '}' in object")),
                None => return Err(self.error("Unterminated object")),
            }
        }
    }
}

/// Create expanded json module builtins
pub fn json_builtins_expanded() -> Vec<PyBuiltinFunction> {
    vec![
        // dumps - serialize object to JSON string with options
        PyBuiltinFunction::new("dumps", |args| {
            let value = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("1 argument", "0 arguments")),
            };

            let mut options = JsonEncodeOptions::default();

            // Parse optional arguments (simplified - in real impl would use kwargs)
            // args[1] = indent, args[2] = separators, args[3] = sort_keys, args[4] = ensure_ascii
            if let Some(PyValue::Int(indent)) = args.get(1) {
                if *indent >= 0 {
                    options.indent = Some(*indent as usize);
                }
            }

            if let Some(PyValue::Tuple(seps)) = args.get(2) {
                let sep_vec = seps.to_vec();
                if sep_vec.len() == 2 {
                    if let (PyValue::Str(item_sep), PyValue::Str(key_sep)) =
                        (&sep_vec[0], &sep_vec[1])
                    {
                        options.separators = Some((item_sep.to_string(), key_sep.to_string()));
                    }
                }
            }

            if let Some(PyValue::Bool(sort)) = args.get(3) {
                options.sort_keys = *sort;
            }

            if let Some(PyValue::Bool(ascii)) = args.get(4) {
                options.ensure_ascii = *ascii;
            }

            let json = value_to_json_with_options(value, &options, 0)?;
            Ok(PyValue::Str(Arc::from(json)))
        }),
        // loads - parse JSON string to object
        PyBuiltinFunction::new("loads", |args| match args.first() {
            Some(PyValue::Str(s)) => json_to_value_nested(s),
            _ => Err(RuntimeError::type_error(
                "str",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // dump - serialize object to file
        PyBuiltinFunction::new("dump", |args| {
            let value = match args.first() {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("2 arguments", "0 arguments")),
            };

            let _file = match args.get(1) {
                Some(f) => f,
                None => return Err(RuntimeError::type_error("2 arguments", "1 argument")),
            };

            // For now, just return the JSON string (file writing would need file object support)
            let json = value_to_json_with_options(value, &JsonEncodeOptions::default(), 0)?;
            Ok(PyValue::Str(Arc::from(json)))
        }),
        // load - parse JSON from file
        PyBuiltinFunction::new("load", |args| match args.first() {
            Some(PyValue::Str(s)) => json_to_value_nested(s),
            _ => Err(RuntimeError::type_error(
                "file",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // JSONEncoder - encoder class
        PyBuiltinFunction::new("JSONEncoder", |args| {
            let encoder = PyDict::new();
            encoder.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("JSONEncoder")),
            );

            // Parse options
            let mut indent = PyValue::None;
            let mut separators = PyValue::None;
            let mut sort_keys = PyValue::Bool(false);
            let mut ensure_ascii = PyValue::Bool(true);

            if let Some(PyValue::Int(i)) = args.first() {
                indent = PyValue::Int(*i);
            }
            if let Some(seps) = args.get(1) {
                separators = seps.clone();
            }
            if let Some(PyValue::Bool(b)) = args.get(2) {
                sort_keys = PyValue::Bool(*b);
            }
            if let Some(PyValue::Bool(b)) = args.get(3) {
                ensure_ascii = PyValue::Bool(*b);
            }

            encoder.setitem(PyKey::Str(Arc::from("indent")), indent);
            encoder.setitem(PyKey::Str(Arc::from("separators")), separators);
            encoder.setitem(PyKey::Str(Arc::from("sort_keys")), sort_keys);
            encoder.setitem(PyKey::Str(Arc::from("ensure_ascii")), ensure_ascii);

            Ok(PyValue::Dict(Arc::new(encoder)))
        }),
        // JSONEncoder.encode - encode a value
        PyBuiltinFunction::new("JSONEncoder_encode", |args| {
            let encoder = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "JSONEncoder",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let value = match args.get(1) {
                Some(v) => v,
                None => return Err(RuntimeError::type_error("2 arguments", "1 argument")),
            };

            let mut options = JsonEncodeOptions::default();

            if let PyValue::Int(i) = encoder.get(&PyKey::Str(Arc::from("indent")), PyValue::None) {
                if i >= 0 {
                    options.indent = Some(i as usize);
                }
            }

            if let PyValue::Bool(b) =
                encoder.get(&PyKey::Str(Arc::from("sort_keys")), PyValue::Bool(false))
            {
                options.sort_keys = b;
            }

            if let PyValue::Bool(b) =
                encoder.get(&PyKey::Str(Arc::from("ensure_ascii")), PyValue::Bool(true))
            {
                options.ensure_ascii = b;
            }

            let json = value_to_json_with_options(value, &options, 0)?;
            Ok(PyValue::Str(Arc::from(json)))
        }),
        // JSONDecoder - decoder class
        PyBuiltinFunction::new("JSONDecoder", |args| {
            let decoder = PyDict::new();
            decoder.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("JSONDecoder")),
            );

            // object_hook is a callable that transforms decoded objects
            let object_hook = args.first().cloned().unwrap_or(PyValue::None);
            decoder.setitem(PyKey::Str(Arc::from("object_hook")), object_hook);

            // object_pairs_hook receives list of pairs instead of dict
            let object_pairs_hook = args.get(1).cloned().unwrap_or(PyValue::None);
            decoder.setitem(PyKey::Str(Arc::from("object_pairs_hook")), object_pairs_hook);

            Ok(PyValue::Dict(Arc::new(decoder)))
        }),
        // JSONDecoder.decode - decode a string
        PyBuiltinFunction::new("JSONDecoder_decode", |args| {
            let _decoder = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "JSONDecoder",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let s = match args.get(1) {
                Some(PyValue::Str(s)) => s,
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            // Note: object_hook would need callable support to fully implement
            json_to_value_nested(s)
        }),
    ]
}

// ===== re module (Task 9.3) =====

/// Get the re module as a dict
pub fn re_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("re")));

    // Regex flags
    dict.setitem(PyKey::Str(Arc::from("IGNORECASE")), PyValue::Int(2));
    dict.setitem(PyKey::Str(Arc::from("I")), PyValue::Int(2));
    dict.setitem(PyKey::Str(Arc::from("MULTILINE")), PyValue::Int(8));
    dict.setitem(PyKey::Str(Arc::from("M")), PyValue::Int(8));
    dict.setitem(PyKey::Str(Arc::from("DOTALL")), PyValue::Int(16));
    dict.setitem(PyKey::Str(Arc::from("S")), PyValue::Int(16));
    dict.setitem(PyKey::Str(Arc::from("VERBOSE")), PyValue::Int(64));
    dict.setitem(PyKey::Str(Arc::from("X")), PyValue::Int(64));
    dict.setitem(PyKey::Str(Arc::from("ASCII")), PyValue::Int(256));
    dict.setitem(PyKey::Str(Arc::from("A")), PyValue::Int(256));

    Arc::new(dict)
}

/// Build regex from pattern and flags
fn build_regex(pattern: &str, flags: i64) -> RuntimeResult<regex::Regex> {
    let mut regex_pattern = String::new();

    // Handle flags
    if flags != 0 {
        regex_pattern.push_str("(?");
        if flags & 2 != 0 {
            regex_pattern.push('i');
        } // IGNORECASE
        if flags & 8 != 0 {
            regex_pattern.push('m');
        } // MULTILINE
        if flags & 16 != 0 {
            regex_pattern.push('s');
        } // DOTALL
        if flags & 64 != 0 {
            regex_pattern.push('x');
        } // VERBOSE
        regex_pattern.push(')');
    }

    regex_pattern.push_str(pattern);

    regex::Regex::new(&regex_pattern)
        .map_err(|e| RuntimeError::value_error(format!("Invalid regex pattern: {}", e)))
}

/// Create a Match object from regex match
fn create_match_object(
    m: regex::Match,
    string: &str,
    pattern: &str,
    captures: Option<&regex::Captures>,
) -> PyValue {
    let match_dict = PyDict::new();
    match_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Match")));
    match_dict.setitem(PyKey::Str(Arc::from("_string")), PyValue::Str(Arc::from(string)));
    match_dict.setitem(PyKey::Str(Arc::from("_pattern")), PyValue::Str(Arc::from(pattern)));
    match_dict.setitem(PyKey::Str(Arc::from("_start")), PyValue::Int(m.start() as i64));
    match_dict.setitem(PyKey::Str(Arc::from("_end")), PyValue::Int(m.end() as i64));
    match_dict.setitem(PyKey::Str(Arc::from("_match")), PyValue::Str(Arc::from(m.as_str())));

    // Store groups
    let groups = if let Some(caps) = captures {
        let mut group_list = Vec::new();
        for i in 0..caps.len() {
            if let Some(g) = caps.get(i) {
                group_list.push(PyValue::Str(Arc::from(g.as_str())));
            } else {
                group_list.push(PyValue::None);
            }
        }
        PyValue::Tuple(Arc::new(PyTuple::from_values(group_list)))
    } else {
        PyValue::Tuple(Arc::new(PyTuple::from_values(vec![PyValue::Str(Arc::from(m.as_str()))])))
    };
    match_dict.setitem(PyKey::Str(Arc::from("_groups")), groups);

    PyValue::Dict(Arc::new(match_dict))
}

/// Create re module builtins
pub fn re_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // compile - compile a regex pattern
        PyBuiltinFunction::new("compile", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let flags = match args.get(1) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            // Validate the pattern compiles
            let _ = build_regex(&pattern, flags)?;

            // Return a Pattern object
            let pattern_dict = PyDict::new();
            pattern_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Pattern")));
            pattern_dict
                .setitem(PyKey::Str(Arc::from("pattern")), PyValue::Str(Arc::from(pattern)));
            pattern_dict.setitem(PyKey::Str(Arc::from("flags")), PyValue::Int(flags));

            Ok(PyValue::Dict(Arc::new(pattern_dict)))
        }),
        // match - match at the beginning of string
        PyBuiltinFunction::new("match", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    // Pattern object
                    match d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::type_error(
                                "str or Pattern",
                                "invalid Pattern",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Pattern",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let string = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let flags = match args.get(2) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            // Build regex with ^ anchor for match behavior
            let anchored_pattern = format!("^(?:{})", pattern);
            let re = build_regex(&anchored_pattern, flags)?;

            match re.captures(&string) {
                Some(caps) => {
                    if let Some(m) = caps.get(0) {
                        Ok(create_match_object(m, &string, &pattern, Some(&caps)))
                    } else {
                        Ok(PyValue::None)
                    }
                }
                None => Ok(PyValue::None),
            }
        }),
        // search - search for pattern anywhere in string
        PyBuiltinFunction::new("search", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    match d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::type_error(
                                "str or Pattern",
                                "invalid Pattern",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Pattern",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let string = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let flags = match args.get(2) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            let re = build_regex(&pattern, flags)?;

            match re.captures(&string) {
                Some(caps) => {
                    if let Some(m) = caps.get(0) {
                        Ok(create_match_object(m, &string, &pattern, Some(&caps)))
                    } else {
                        Ok(PyValue::None)
                    }
                }
                None => Ok(PyValue::None),
            }
        }),
        // findall - find all non-overlapping matches
        PyBuiltinFunction::new("findall", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    match d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::type_error(
                                "str or Pattern",
                                "invalid Pattern",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Pattern",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let string = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let flags = match args.get(2) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            let re = build_regex(&pattern, flags)?;

            let matches: Vec<PyValue> = re
                .captures_iter(&string)
                .map(|caps| {
                    if caps.len() > 1 {
                        // Has groups - return tuple of groups (excluding group 0)
                        let groups: Vec<PyValue> = (1..caps.len())
                            .map(|i| {
                                caps.get(i)
                                    .map(|m| PyValue::Str(Arc::from(m.as_str())))
                                    .unwrap_or(PyValue::None)
                            })
                            .collect();
                        if groups.len() == 1 {
                            groups.into_iter().next().unwrap()
                        } else {
                            PyValue::Tuple(Arc::new(PyTuple::from_values(groups)))
                        }
                    } else {
                        // No groups - return the match
                        caps.get(0)
                            .map(|m| PyValue::Str(Arc::from(m.as_str())))
                            .unwrap_or(PyValue::None)
                    }
                })
                .collect();

            Ok(PyValue::List(Arc::new(crate::PyList::from_values(matches))))
        }),
        // finditer - return iterator of match objects
        PyBuiltinFunction::new("finditer", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    match d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::type_error(
                                "str or Pattern",
                                "invalid Pattern",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Pattern",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let string = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let flags = match args.get(2) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            let re = build_regex(&pattern, flags)?;

            // Return list of match objects (simplified - real impl would be iterator)
            let matches: Vec<PyValue> = re
                .captures_iter(&string)
                .filter_map(|caps| {
                    caps.get(0).map(|m| create_match_object(m, &string, &pattern, Some(&caps)))
                })
                .collect();

            Ok(PyValue::List(Arc::new(crate::PyList::from_values(matches))))
        }),
        // sub - replace pattern with replacement
        PyBuiltinFunction::new("sub", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    match d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::type_error(
                                "str or Pattern",
                                "invalid Pattern",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Pattern",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let repl = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let string = match args.get(2) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(2).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let count = match args.get(3) {
                Some(PyValue::Int(n)) => *n as usize,
                _ => 0, // 0 means replace all
            };

            let flags = match args.get(4) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            let re = build_regex(&pattern, flags)?;

            // Convert Python-style backreferences (\1, \2) to Rust-style ($1, $2)
            let rust_repl = repl
                .replace("\\1", "$1")
                .replace("\\2", "$2")
                .replace("\\3", "$3")
                .replace("\\4", "$4")
                .replace("\\5", "$5")
                .replace("\\6", "$6")
                .replace("\\7", "$7")
                .replace("\\8", "$8")
                .replace("\\9", "$9")
                .replace("\\g<", "${")
                .replace(">", "}");

            let result = if count == 0 {
                re.replace_all(&string, rust_repl.as_str()).to_string()
            } else {
                re.replacen(&string, count, rust_repl.as_str()).to_string()
            };

            Ok(PyValue::Str(Arc::from(result)))
        }),
        // subn - replace pattern and return (new_string, count)
        PyBuiltinFunction::new("subn", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    match d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::type_error(
                                "str or Pattern",
                                "invalid Pattern",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Pattern",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let repl = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let string = match args.get(2) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(2).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let max_count = match args.get(3) {
                Some(PyValue::Int(n)) => *n as usize,
                _ => 0,
            };

            let flags = match args.get(4) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            let re = build_regex(&pattern, flags)?;

            // Convert backreferences
            let rust_repl = repl
                .replace("\\1", "$1")
                .replace("\\2", "$2")
                .replace("\\3", "$3")
                .replace("\\4", "$4")
                .replace("\\5", "$5")
                .replace("\\6", "$6")
                .replace("\\7", "$7")
                .replace("\\8", "$8")
                .replace("\\9", "$9");

            // Count matches
            let match_count = re.find_iter(&string).count();
            let actual_count = if max_count == 0 {
                match_count
            } else {
                match_count.min(max_count)
            };

            let result = if max_count == 0 {
                re.replace_all(&string, rust_repl.as_str()).to_string()
            } else {
                re.replacen(&string, max_count, rust_repl.as_str()).to_string()
            };

            Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                PyValue::Str(Arc::from(result)),
                PyValue::Int(actual_count as i64),
            ]))))
        }),
        // split - split string by pattern
        PyBuiltinFunction::new("split", |args| {
            let pattern = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    match d.get(&PyKey::Str(Arc::from("pattern")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::type_error(
                                "str or Pattern",
                                "invalid Pattern",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Pattern",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let string = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let maxsplit = match args.get(2) {
                Some(PyValue::Int(n)) => *n as usize,
                _ => 0,
            };

            let flags = match args.get(3) {
                Some(PyValue::Int(f)) => *f,
                _ => 0,
            };

            let re = build_regex(&pattern, flags)?;

            let parts: Vec<PyValue> = if maxsplit == 0 {
                re.split(&string).map(|s| PyValue::Str(Arc::from(s))).collect()
            } else {
                re.splitn(&string, maxsplit + 1).map(|s| PyValue::Str(Arc::from(s))).collect()
            };

            Ok(PyValue::List(Arc::new(crate::PyList::from_values(parts))))
        }),
        // escape - escape special regex characters
        PyBuiltinFunction::new("escape", |args| {
            let string = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            Ok(PyValue::Str(Arc::from(regex::escape(&string))))
        }),
        // Match object methods
        PyBuiltinFunction::new("Match_group", |args| {
            let match_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Match",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let groups = match match_obj.get(&PyKey::Str(Arc::from("_groups")), PyValue::None) {
                PyValue::Tuple(t) => t,
                _ => return Err(RuntimeError::internal_error("Invalid Match object")),
            };

            let group_num = match args.get(1) {
                Some(PyValue::Int(n)) => *n as usize,
                None => 0,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let group_vec = groups.to_vec();
            if group_num >= group_vec.len() {
                return Err(RuntimeError::index_error(group_num as i64, group_vec.len()));
            }

            Ok(group_vec[group_num].clone())
        }),
        PyBuiltinFunction::new("Match_groups", |args| {
            let match_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Match",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let groups = match match_obj.get(&PyKey::Str(Arc::from("_groups")), PyValue::None) {
                PyValue::Tuple(t) => t,
                _ => return Err(RuntimeError::internal_error("Invalid Match object")),
            };

            // Return all groups except group 0
            let group_vec = groups.to_vec();
            if group_vec.len() <= 1 {
                return Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![]))));
            }

            let default = match args.get(1) {
                Some(v) => v.clone(),
                None => PyValue::None,
            };

            let result: Vec<PyValue> = group_vec[1..]
                .iter()
                .map(|g| {
                    if matches!(g, PyValue::None) {
                        default.clone()
                    } else {
                        g.clone()
                    }
                })
                .collect();

            Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(result))))
        }),
        PyBuiltinFunction::new("Match_span", |args| {
            let match_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Match",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let start = match match_obj.get(&PyKey::Str(Arc::from("_start")), PyValue::None) {
                PyValue::Int(n) => n,
                _ => return Err(RuntimeError::internal_error("Invalid Match object")),
            };

            let end = match match_obj.get(&PyKey::Str(Arc::from("_end")), PyValue::None) {
                PyValue::Int(n) => n,
                _ => return Err(RuntimeError::internal_error("Invalid Match object")),
            };

            Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                PyValue::Int(start),
                PyValue::Int(end),
            ]))))
        }),
        PyBuiltinFunction::new("Match_start", |args| {
            let match_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Match",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            match match_obj.get(&PyKey::Str(Arc::from("_start")), PyValue::None) {
                PyValue::Int(n) => Ok(PyValue::Int(n)),
                _ => Err(RuntimeError::internal_error("Invalid Match object")),
            }
        }),
        PyBuiltinFunction::new("Match_end", |args| {
            let match_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Match",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            match match_obj.get(&PyKey::Str(Arc::from("_end")), PyValue::None) {
                PyValue::Int(n) => Ok(PyValue::Int(n)),
                _ => Err(RuntimeError::internal_error("Invalid Match object")),
            }
        }),
    ]
}

// ===== datetime module (Task 10.1) =====

/// Get the datetime module as a dict
pub fn datetime_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("datetime")));

    // Constants
    dict.setitem(PyKey::Str(Arc::from("MINYEAR")), PyValue::Int(1));
    dict.setitem(PyKey::Str(Arc::from("MAXYEAR")), PyValue::Int(9999));

    Arc::new(dict)
}

/// Helper to get current time components
fn get_current_time() -> (i64, i64, i64, i64, i64, i64, i64) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();

    let total_secs = now.as_secs() as i64;
    let micros = now.subsec_micros() as i64;

    // Calculate date/time components (simplified - doesn't handle leap seconds)
    let days_since_epoch = total_secs / 86400;
    let secs_today = total_secs % 86400;

    let hour = secs_today / 3600;
    let minute = (secs_today % 3600) / 60;
    let second = secs_today % 60;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days_since_epoch + 719468); // Days since year 0

    (year, month, day, hour, minute, second, micros)
}

/// Convert days since year 0 to (year, month, day)
fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    // Algorithm from Howard Hinnant's date algorithms
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as i64, d as i64)
}

/// Convert (year, month, day) to days since year 0
fn ymd_to_days(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = month as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as u32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64
}

/// Check if year is a leap year
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Get days in month
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Create datetime module builtins
pub fn datetime_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // date class constructor
        PyBuiltinFunction::new("date", |args| {
            let year = match args.first() {
                Some(PyValue::Int(y)) => *y,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let month = match args.get(1) {
                Some(PyValue::Int(m)) => *m,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let day = match args.get(2) {
                Some(PyValue::Int(d)) => *d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.get(2).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            // Validate
            if year < 1 || year > 9999 {
                return Err(RuntimeError::value_error("year is out of range"));
            }
            if month < 1 || month > 12 {
                return Err(RuntimeError::value_error("month must be in 1..12"));
            }
            let max_day = days_in_month(year, month);
            if day < 1 || day > max_day {
                return Err(RuntimeError::value_error(format!("day is out of range for month")));
            }

            let date_dict = PyDict::new();
            date_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("date")));
            date_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            date_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            date_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));

            Ok(PyValue::Dict(Arc::new(date_dict)))
        }),
        // date.today() - class method
        PyBuiltinFunction::new("date_today", |_args| {
            let (year, month, day, _, _, _, _) = get_current_time();

            let date_dict = PyDict::new();
            date_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("date")));
            date_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            date_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            date_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));

            Ok(PyValue::Dict(Arc::new(date_dict)))
        }),
        // date.fromtimestamp(timestamp)
        PyBuiltinFunction::new("date_fromtimestamp", |args| {
            let timestamp = match args.first() {
                Some(PyValue::Int(t)) => *t,
                Some(PyValue::Float(t)) => *t as i64,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int or float",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let days = timestamp / 86400 + 719468;
            let (year, month, day) = days_to_ymd(days);

            let date_dict = PyDict::new();
            date_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("date")));
            date_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            date_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            date_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));

            Ok(PyValue::Dict(Arc::new(date_dict)))
        }),
        // time class constructor
        PyBuiltinFunction::new("time", |args| {
            let hour = match args.first() {
                Some(PyValue::Int(h)) => *h,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[0].type_name())),
            };
            let minute = match args.get(1) {
                Some(PyValue::Int(m)) => *m,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[1].type_name())),
            };
            let second = match args.get(2) {
                Some(PyValue::Int(s)) => *s,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[2].type_name())),
            };
            let microsecond = match args.get(3) {
                Some(PyValue::Int(us)) => *us,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[3].type_name())),
            };

            // Validate
            if hour < 0 || hour > 23 {
                return Err(RuntimeError::value_error("hour must be in 0..23"));
            }
            if minute < 0 || minute > 59 {
                return Err(RuntimeError::value_error("minute must be in 0..59"));
            }
            if second < 0 || second > 59 {
                return Err(RuntimeError::value_error("second must be in 0..59"));
            }
            if microsecond < 0 || microsecond > 999999 {
                return Err(RuntimeError::value_error("microsecond must be in 0..999999"));
            }

            let time_dict = PyDict::new();
            time_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("time")));
            time_dict.setitem(PyKey::Str(Arc::from("hour")), PyValue::Int(hour));
            time_dict.setitem(PyKey::Str(Arc::from("minute")), PyValue::Int(minute));
            time_dict.setitem(PyKey::Str(Arc::from("second")), PyValue::Int(second));
            time_dict.setitem(PyKey::Str(Arc::from("microsecond")), PyValue::Int(microsecond));

            Ok(PyValue::Dict(Arc::new(time_dict)))
        }),
        // datetime class constructor
        PyBuiltinFunction::new("datetime", |args| {
            let year = match args.first() {
                Some(PyValue::Int(y)) => *y,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let month = match args.get(1) {
                Some(PyValue::Int(m)) => *m,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let day = match args.get(2) {
                Some(PyValue::Int(d)) => *d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.get(2).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let hour = match args.get(3) {
                Some(PyValue::Int(h)) => *h,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[3].type_name())),
            };
            let minute = match args.get(4) {
                Some(PyValue::Int(m)) => *m,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[4].type_name())),
            };
            let second = match args.get(5) {
                Some(PyValue::Int(s)) => *s,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[5].type_name())),
            };
            let microsecond = match args.get(6) {
                Some(PyValue::Int(us)) => *us,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[6].type_name())),
            };

            // Validate date
            if year < 1 || year > 9999 {
                return Err(RuntimeError::value_error("year is out of range"));
            }
            if month < 1 || month > 12 {
                return Err(RuntimeError::value_error("month must be in 1..12"));
            }
            let max_day = days_in_month(year, month);
            if day < 1 || day > max_day {
                return Err(RuntimeError::value_error("day is out of range for month"));
            }

            // Validate time
            if hour < 0 || hour > 23 {
                return Err(RuntimeError::value_error("hour must be in 0..23"));
            }
            if minute < 0 || minute > 59 {
                return Err(RuntimeError::value_error("minute must be in 0..59"));
            }
            if second < 0 || second > 59 {
                return Err(RuntimeError::value_error("second must be in 0..59"));
            }
            if microsecond < 0 || microsecond > 999999 {
                return Err(RuntimeError::value_error("microsecond must be in 0..999999"));
            }

            let dt_dict = PyDict::new();
            dt_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("datetime")));
            dt_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            dt_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            dt_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));
            dt_dict.setitem(PyKey::Str(Arc::from("hour")), PyValue::Int(hour));
            dt_dict.setitem(PyKey::Str(Arc::from("minute")), PyValue::Int(minute));
            dt_dict.setitem(PyKey::Str(Arc::from("second")), PyValue::Int(second));
            dt_dict.setitem(PyKey::Str(Arc::from("microsecond")), PyValue::Int(microsecond));

            Ok(PyValue::Dict(Arc::new(dt_dict)))
        }),
        // datetime.now()
        PyBuiltinFunction::new("datetime_now", |_args| {
            let (year, month, day, hour, minute, second, microsecond) = get_current_time();

            let dt_dict = PyDict::new();
            dt_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("datetime")));
            dt_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            dt_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            dt_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));
            dt_dict.setitem(PyKey::Str(Arc::from("hour")), PyValue::Int(hour));
            dt_dict.setitem(PyKey::Str(Arc::from("minute")), PyValue::Int(minute));
            dt_dict.setitem(PyKey::Str(Arc::from("second")), PyValue::Int(second));
            dt_dict.setitem(PyKey::Str(Arc::from("microsecond")), PyValue::Int(microsecond));

            Ok(PyValue::Dict(Arc::new(dt_dict)))
        }),
        // datetime.utcnow()
        PyBuiltinFunction::new("datetime_utcnow", |_args| {
            let (year, month, day, hour, minute, second, microsecond) = get_current_time();

            let dt_dict = PyDict::new();
            dt_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("datetime")));
            dt_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            dt_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            dt_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));
            dt_dict.setitem(PyKey::Str(Arc::from("hour")), PyValue::Int(hour));
            dt_dict.setitem(PyKey::Str(Arc::from("minute")), PyValue::Int(minute));
            dt_dict.setitem(PyKey::Str(Arc::from("second")), PyValue::Int(second));
            dt_dict.setitem(PyKey::Str(Arc::from("microsecond")), PyValue::Int(microsecond));

            Ok(PyValue::Dict(Arc::new(dt_dict)))
        }),
        // datetime.fromtimestamp(timestamp)
        PyBuiltinFunction::new("datetime_fromtimestamp", |args| {
            let timestamp = match args.first() {
                Some(PyValue::Int(t)) => *t as f64,
                Some(PyValue::Float(t)) => *t,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int or float",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let total_secs = timestamp as i64;
            let micros = ((timestamp - total_secs as f64) * 1_000_000.0) as i64;

            let days = total_secs / 86400 + 719468;
            let secs_today = total_secs % 86400;

            let (year, month, day) = days_to_ymd(days);
            let hour = secs_today / 3600;
            let minute = (secs_today % 3600) / 60;
            let second = secs_today % 60;

            let dt_dict = PyDict::new();
            dt_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("datetime")));
            dt_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            dt_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            dt_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));
            dt_dict.setitem(PyKey::Str(Arc::from("hour")), PyValue::Int(hour));
            dt_dict.setitem(PyKey::Str(Arc::from("minute")), PyValue::Int(minute));
            dt_dict.setitem(PyKey::Str(Arc::from("second")), PyValue::Int(second));
            dt_dict.setitem(PyKey::Str(Arc::from("microsecond")), PyValue::Int(micros.abs()));

            Ok(PyValue::Dict(Arc::new(dt_dict)))
        }),
        // timedelta constructor
        PyBuiltinFunction::new("timedelta", |args| {
            let days = match args.first() {
                Some(PyValue::Int(d)) => *d,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[0].type_name())),
            };
            let seconds = match args.get(1) {
                Some(PyValue::Int(s)) => *s,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[1].type_name())),
            };
            let microseconds = match args.get(2) {
                Some(PyValue::Int(us)) => *us,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[2].type_name())),
            };
            let milliseconds = match args.get(3) {
                Some(PyValue::Int(ms)) => *ms,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[3].type_name())),
            };
            let minutes = match args.get(4) {
                Some(PyValue::Int(m)) => *m,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[4].type_name())),
            };
            let hours = match args.get(5) {
                Some(PyValue::Int(h)) => *h,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[5].type_name())),
            };
            let weeks = match args.get(6) {
                Some(PyValue::Int(w)) => *w,
                None => 0,
                _ => return Err(RuntimeError::type_error("int", args[6].type_name())),
            };

            // Normalize to days, seconds, microseconds
            let total_us = microseconds + milliseconds * 1000;
            let total_secs = seconds + minutes * 60 + hours * 3600 + (total_us / 1_000_000);
            let final_us = total_us % 1_000_000;
            let total_days = days + weeks * 7 + (total_secs / 86400);
            let final_secs = total_secs % 86400;

            let td_dict = PyDict::new();
            td_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("timedelta")));
            td_dict.setitem(PyKey::Str(Arc::from("days")), PyValue::Int(total_days));
            td_dict.setitem(PyKey::Str(Arc::from("seconds")), PyValue::Int(final_secs));
            td_dict.setitem(PyKey::Str(Arc::from("microseconds")), PyValue::Int(final_us));

            Ok(PyValue::Dict(Arc::new(td_dict)))
        }),
        // timedelta.total_seconds()
        PyBuiltinFunction::new("timedelta_total_seconds", |args| {
            let td = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "timedelta",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let days = match td.get(&PyKey::Str(Arc::from("days")), PyValue::Int(0)) {
                PyValue::Int(d) => d,
                _ => 0,
            };
            let seconds = match td.get(&PyKey::Str(Arc::from("seconds")), PyValue::Int(0)) {
                PyValue::Int(s) => s,
                _ => 0,
            };
            let microseconds = match td.get(&PyKey::Str(Arc::from("microseconds")), PyValue::Int(0))
            {
                PyValue::Int(us) => us,
                _ => 0,
            };

            let total = days as f64 * 86400.0 + seconds as f64 + microseconds as f64 / 1_000_000.0;
            Ok(PyValue::Float(total))
        }),
        // date/datetime.isoformat()
        PyBuiltinFunction::new("date_isoformat", |args| {
            let obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "date or datetime",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let year = match obj.get(&PyKey::Str(Arc::from("year")), PyValue::Int(0)) {
                PyValue::Int(y) => y,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };
            let month = match obj.get(&PyKey::Str(Arc::from("month")), PyValue::Int(0)) {
                PyValue::Int(m) => m,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };
            let day = match obj.get(&PyKey::Str(Arc::from("day")), PyValue::Int(0)) {
                PyValue::Int(d) => d,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };

            Ok(PyValue::Str(Arc::from(format!("{:04}-{:02}-{:02}", year, month, day))))
        }),
        // datetime.isoformat()
        PyBuiltinFunction::new("datetime_isoformat", |args| {
            let obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "datetime",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let year = match obj.get(&PyKey::Str(Arc::from("year")), PyValue::Int(0)) {
                PyValue::Int(y) => y,
                _ => return Err(RuntimeError::internal_error("Invalid datetime object")),
            };
            let month = match obj.get(&PyKey::Str(Arc::from("month")), PyValue::Int(0)) {
                PyValue::Int(m) => m,
                _ => return Err(RuntimeError::internal_error("Invalid datetime object")),
            };
            let day = match obj.get(&PyKey::Str(Arc::from("day")), PyValue::Int(0)) {
                PyValue::Int(d) => d,
                _ => return Err(RuntimeError::internal_error("Invalid datetime object")),
            };
            let hour = match obj.get(&PyKey::Str(Arc::from("hour")), PyValue::Int(0)) {
                PyValue::Int(h) => h,
                _ => 0,
            };
            let minute = match obj.get(&PyKey::Str(Arc::from("minute")), PyValue::Int(0)) {
                PyValue::Int(m) => m,
                _ => 0,
            };
            let second = match obj.get(&PyKey::Str(Arc::from("second")), PyValue::Int(0)) {
                PyValue::Int(s) => s,
                _ => 0,
            };
            let microsecond = match obj.get(&PyKey::Str(Arc::from("microsecond")), PyValue::Int(0))
            {
                PyValue::Int(us) => us,
                _ => 0,
            };

            let sep = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => "T".to_string(),
            };

            let iso = if microsecond > 0 {
                format!(
                    "{:04}-{:02}-{:02}{}{:02}:{:02}:{:02}.{:06}",
                    year, month, day, sep, hour, minute, second, microsecond
                )
            } else {
                format!(
                    "{:04}-{:02}-{:02}{}{:02}:{:02}:{:02}",
                    year, month, day, sep, hour, minute, second
                )
            };

            Ok(PyValue::Str(Arc::from(iso)))
        }),
        // date/datetime.strftime(format)
        PyBuiltinFunction::new("strftime", |args| {
            let obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "date or datetime",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let format = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let year = match obj.get(&PyKey::Str(Arc::from("year")), PyValue::Int(0)) {
                PyValue::Int(y) => y,
                _ => 0,
            };
            let month = match obj.get(&PyKey::Str(Arc::from("month")), PyValue::Int(0)) {
                PyValue::Int(m) => m,
                _ => 0,
            };
            let day = match obj.get(&PyKey::Str(Arc::from("day")), PyValue::Int(0)) {
                PyValue::Int(d) => d,
                _ => 0,
            };
            let hour = match obj.get(&PyKey::Str(Arc::from("hour")), PyValue::Int(0)) {
                PyValue::Int(h) => h,
                _ => 0,
            };
            let minute = match obj.get(&PyKey::Str(Arc::from("minute")), PyValue::Int(0)) {
                PyValue::Int(m) => m,
                _ => 0,
            };
            let second = match obj.get(&PyKey::Str(Arc::from("second")), PyValue::Int(0)) {
                PyValue::Int(s) => s,
                _ => 0,
            };
            let microsecond = match obj.get(&PyKey::Str(Arc::from("microsecond")), PyValue::Int(0))
            {
                PyValue::Int(us) => us,
                _ => 0,
            };

            // Calculate weekday (0=Monday, 6=Sunday)
            let days = ymd_to_days(year, month, day);
            let weekday = ((days + 3) % 7) as i64; // 1970-01-01 was Thursday (3)

            // Day names
            let day_names = [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ];
            let day_abbrs = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            let month_names = [
                "",
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ];
            let month_abbrs = [
                "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                "Dec",
            ];

            // Process format string
            let mut result = String::new();
            let mut chars = format.chars().peekable();

            while let Some(c) = chars.next() {
                if c == '%' {
                    if let Some(&spec) = chars.peek() {
                        chars.next();
                        match spec {
                            'Y' => result.push_str(&format!("{:04}", year)),
                            'y' => result.push_str(&format!("{:02}", year % 100)),
                            'm' => result.push_str(&format!("{:02}", month)),
                            'd' => result.push_str(&format!("{:02}", day)),
                            'H' => result.push_str(&format!("{:02}", hour)),
                            'M' => result.push_str(&format!("{:02}", minute)),
                            'S' => result.push_str(&format!("{:02}", second)),
                            'f' => result.push_str(&format!("{:06}", microsecond)),
                            'A' => result.push_str(day_names[weekday as usize]),
                            'a' => result.push_str(day_abbrs[weekday as usize]),
                            'B' => result.push_str(month_names[month as usize]),
                            'b' => result.push_str(month_abbrs[month as usize]),
                            'w' => result.push_str(&format!("{}", (weekday + 1) % 7)), // Sunday=0
                            'j' => {
                                // Day of year
                                let mut doy = day;
                                for m in 1..month {
                                    doy += days_in_month(year, m);
                                }
                                result.push_str(&format!("{:03}", doy));
                            }
                            'I' => result.push_str(&format!(
                                "{:02}",
                                if hour == 0 {
                                    12
                                } else if hour > 12 {
                                    hour - 12
                                } else {
                                    hour
                                }
                            )),
                            'p' => result.push_str(if hour < 12 { "AM" } else { "PM" }),
                            '%' => result.push('%'),
                            _ => {
                                result.push('%');
                                result.push(spec);
                            }
                        }
                    } else {
                        result.push('%');
                    }
                } else {
                    result.push(c);
                }
            }

            Ok(PyValue::Str(Arc::from(result)))
        }),
        // datetime.strptime(string, format)
        PyBuiltinFunction::new("datetime_strptime", |args| {
            let date_string = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let format = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            // Simple parser for common formats
            let mut year = 1900i64;
            let mut month = 1i64;
            let mut day = 1i64;
            let mut hour = 0i64;
            let mut minute = 0i64;
            let mut second = 0i64;
            let mut microsecond = 0i64;

            let mut date_chars = date_string.chars().peekable();
            let mut format_chars = format.chars().peekable();

            while let Some(fc) = format_chars.next() {
                if fc == '%' {
                    if let Some(&spec) = format_chars.peek() {
                        format_chars.next();
                        match spec {
                            'Y' => {
                                let s: String = date_chars.by_ref().take(4).collect();
                                year = s.parse().unwrap_or(1900);
                            }
                            'y' => {
                                let s: String = date_chars.by_ref().take(2).collect();
                                let y: i64 = s.parse().unwrap_or(0);
                                year = if y >= 69 { 1900 + y } else { 2000 + y };
                            }
                            'm' => {
                                let s: String = date_chars.by_ref().take(2).collect();
                                month = s.parse().unwrap_or(1);
                            }
                            'd' => {
                                let s: String = date_chars.by_ref().take(2).collect();
                                day = s.parse().unwrap_or(1);
                            }
                            'H' => {
                                let s: String = date_chars.by_ref().take(2).collect();
                                hour = s.parse().unwrap_or(0);
                            }
                            'M' => {
                                let s: String = date_chars.by_ref().take(2).collect();
                                minute = s.parse().unwrap_or(0);
                            }
                            'S' => {
                                let s: String = date_chars.by_ref().take(2).collect();
                                second = s.parse().unwrap_or(0);
                            }
                            'f' => {
                                let s: String = date_chars.by_ref().take(6).collect();
                                microsecond = s.parse().unwrap_or(0);
                            }
                            '%' => {
                                date_chars.next();
                            }
                            _ => {
                                date_chars.next();
                            }
                        }
                    }
                } else {
                    date_chars.next();
                }
            }

            let dt_dict = PyDict::new();
            dt_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("datetime")));
            dt_dict.setitem(PyKey::Str(Arc::from("year")), PyValue::Int(year));
            dt_dict.setitem(PyKey::Str(Arc::from("month")), PyValue::Int(month));
            dt_dict.setitem(PyKey::Str(Arc::from("day")), PyValue::Int(day));
            dt_dict.setitem(PyKey::Str(Arc::from("hour")), PyValue::Int(hour));
            dt_dict.setitem(PyKey::Str(Arc::from("minute")), PyValue::Int(minute));
            dt_dict.setitem(PyKey::Str(Arc::from("second")), PyValue::Int(second));
            dt_dict.setitem(PyKey::Str(Arc::from("microsecond")), PyValue::Int(microsecond));

            Ok(PyValue::Dict(Arc::new(dt_dict)))
        }),
        // date.weekday() - Monday=0, Sunday=6
        PyBuiltinFunction::new("date_weekday", |args| {
            let obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "date or datetime",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let year = match obj.get(&PyKey::Str(Arc::from("year")), PyValue::Int(0)) {
                PyValue::Int(y) => y,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };
            let month = match obj.get(&PyKey::Str(Arc::from("month")), PyValue::Int(0)) {
                PyValue::Int(m) => m,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };
            let day = match obj.get(&PyKey::Str(Arc::from("day")), PyValue::Int(0)) {
                PyValue::Int(d) => d,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };

            let days = ymd_to_days(year, month, day);
            let weekday = ((days + 3) % 7) as i64;

            Ok(PyValue::Int(weekday))
        }),
        // date.isoweekday() - Monday=1, Sunday=7
        PyBuiltinFunction::new("date_isoweekday", |args| {
            let obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "date or datetime",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let year = match obj.get(&PyKey::Str(Arc::from("year")), PyValue::Int(0)) {
                PyValue::Int(y) => y,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };
            let month = match obj.get(&PyKey::Str(Arc::from("month")), PyValue::Int(0)) {
                PyValue::Int(m) => m,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };
            let day = match obj.get(&PyKey::Str(Arc::from("day")), PyValue::Int(0)) {
                PyValue::Int(d) => d,
                _ => return Err(RuntimeError::internal_error("Invalid date object")),
            };

            let days = ymd_to_days(year, month, day);
            let weekday = ((days + 3) % 7) as i64 + 1;

            Ok(PyValue::Int(weekday))
        }),
    ]
}

// ===== pathlib module (Task 10.2) =====

/// Get the pathlib module as a dict
pub fn pathlib_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("pathlib")));

    Arc::new(dict)
}

/// Create a Path object from a string
fn create_path_object(path: &str) -> PyValue {
    let path_dict = PyDict::new();
    path_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Path")));
    path_dict.setitem(PyKey::Str(Arc::from("_path")), PyValue::Str(Arc::from(path)));

    // Parse path components
    let p = std::path::Path::new(path);

    // Store parts
    let parts: Vec<PyValue> = p
        .components()
        .map(|c| PyValue::Str(Arc::from(c.as_os_str().to_string_lossy().to_string())))
        .collect();
    path_dict.setitem(
        PyKey::Str(Arc::from("parts")),
        PyValue::Tuple(Arc::new(PyTuple::from_values(parts))),
    );

    // Store name (final component)
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    path_dict.setitem(PyKey::Str(Arc::from("name")), PyValue::Str(Arc::from(name)));

    // Store suffix (extension with dot)
    let suffix = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    path_dict.setitem(PyKey::Str(Arc::from("suffix")), PyValue::Str(Arc::from(suffix)));

    // Store stem (name without suffix)
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    path_dict.setitem(PyKey::Str(Arc::from("stem")), PyValue::Str(Arc::from(stem)));

    PyValue::Dict(Arc::new(path_dict))
}

/// Create pathlib module builtins
pub fn pathlib_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // Path constructor
        PyBuiltinFunction::new("Path", |args| {
            let path = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    // If it's already a Path object, get its _path
                    match d.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => ".".to_string(),
                    }
                }
                None => ".".to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            // Join additional path components
            let mut full_path = std::path::PathBuf::from(&path);
            for arg in args.iter().skip(1) {
                match arg {
                    PyValue::Str(s) => full_path.push(s.as_ref()),
                    PyValue::Dict(d) => {
                        if let PyValue::Str(s) =
                            d.get(&PyKey::Str(Arc::from("_path")), PyValue::None)
                        {
                            full_path.push(s.as_ref());
                        }
                    }
                    _ => {}
                }
            }

            Ok(create_path_object(&full_path.to_string_lossy()))
        }),
        // Path.cwd() - class method
        PyBuiltinFunction::new("Path_cwd", |_args| {
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            Ok(create_path_object(&cwd))
        }),
        // Path.home() - class method
        PyBuiltinFunction::new("Path_home", |_args| {
            let home = if cfg!(target_os = "windows") {
                std::env::var("USERPROFILE").ok()
            } else {
                std::env::var("HOME").ok()
            };

            let home_path = home.unwrap_or_else(|| ".".to_string());
            Ok(create_path_object(&home_path))
        }),
        // Path.__truediv__ (/ operator) - joinpath
        PyBuiltinFunction::new("Path_joinpath", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let base_path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let mut full_path = std::path::PathBuf::from(&base_path);

            for arg in args.iter().skip(1) {
                match arg {
                    PyValue::Str(s) => full_path.push(s.as_ref()),
                    PyValue::Dict(d) => {
                        if let PyValue::Str(s) =
                            d.get(&PyKey::Str(Arc::from("_path")), PyValue::None)
                        {
                            full_path.push(s.as_ref());
                        }
                    }
                    _ => {}
                }
            }

            Ok(create_path_object(&full_path.to_string_lossy()))
        }),
        // Path.parent property
        PyBuiltinFunction::new("Path_parent", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let p = std::path::Path::new(&path);
            let parent = p
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());

            Ok(create_path_object(&parent))
        }),
        // Path.exists()
        PyBuiltinFunction::new("Path_exists", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            Ok(PyValue::Bool(std::path::Path::new(&path).exists()))
        }),
        // Path.is_file()
        PyBuiltinFunction::new("Path_is_file", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            Ok(PyValue::Bool(std::path::Path::new(&path).is_file()))
        }),
        // Path.is_dir()
        PyBuiltinFunction::new("Path_is_dir", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            Ok(PyValue::Bool(std::path::Path::new(&path).is_dir()))
        }),
        // Path.is_absolute()
        PyBuiltinFunction::new("Path_is_absolute", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            Ok(PyValue::Bool(std::path::Path::new(&path).is_absolute()))
        }),
        // Path.resolve()
        PyBuiltinFunction::new("Path_resolve", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let resolved = std::fs::canonicalize(&path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| {
                    // If can't canonicalize, at least make it absolute
                    let p = std::path::Path::new(&path);
                    if p.is_absolute() {
                        path.clone()
                    } else {
                        std::env::current_dir()
                            .map(|cwd| cwd.join(p).to_string_lossy().to_string())
                            .unwrap_or(path.clone())
                    }
                });

            Ok(create_path_object(&resolved))
        }),
        // Path.iterdir()
        PyBuiltinFunction::new("Path_iterdir", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let entries: RuntimeResult<Vec<PyValue>> = std::fs::read_dir(&path)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot iterate directory '{}': {}", path, e),
                })?
                .map(|entry| {
                    entry.map(|e| create_path_object(&e.path().to_string_lossy())).map_err(|e| {
                        RuntimeError::OsError {
                            message: e.to_string(),
                        }
                    })
                })
                .collect();

            Ok(PyValue::List(Arc::new(crate::PyList::from_values(entries?))))
        }),
        // Path.glob(pattern)
        PyBuiltinFunction::new("Path_glob", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let base_path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let pattern = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            // Simple glob implementation - just handle * and **
            let full_pattern = std::path::Path::new(&base_path).join(&pattern);
            let pattern_str = full_pattern.to_string_lossy();

            // For now, just list directory if pattern is simple
            if pattern == "*" {
                let entries: RuntimeResult<Vec<PyValue>> = std::fs::read_dir(&base_path)
                    .map_err(|e| RuntimeError::OsError {
                        message: format!("Cannot glob '{}': {}", pattern_str, e),
                    })?
                    .filter_map(|entry| entry.ok())
                    .map(|e| Ok(create_path_object(&e.path().to_string_lossy())))
                    .collect();
                return Ok(PyValue::List(Arc::new(crate::PyList::from_values(entries?))));
            }

            // For other patterns, return empty list (simplified)
            Ok(PyValue::List(Arc::new(crate::PyList::new())))
        }),
        // Path.mkdir(parents=False, exist_ok=False)
        PyBuiltinFunction::new("Path_mkdir", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let parents = match args.get(1) {
                Some(PyValue::Bool(b)) => *b,
                _ => false,
            };

            let exist_ok = match args.get(2) {
                Some(PyValue::Bool(b)) => *b,
                _ => false,
            };

            let p = std::path::Path::new(&path);

            if exist_ok && p.exists() {
                return Ok(PyValue::None);
            }

            let result = if parents {
                std::fs::create_dir_all(&path)
            } else {
                std::fs::create_dir(&path)
            };

            result.map(|_| PyValue::None).map_err(|e| RuntimeError::OsError {
                message: format!("Cannot create directory '{}': {}", path, e),
            })
        }),
        // Path.rmdir()
        PyBuiltinFunction::new("Path_rmdir", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            std::fs::remove_dir(&path)
                .map(|_| PyValue::None)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot remove directory '{}': {}", path, e),
                })
        }),
        // Path.unlink(missing_ok=False)
        PyBuiltinFunction::new("Path_unlink", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let missing_ok = match args.get(1) {
                Some(PyValue::Bool(b)) => *b,
                _ => false,
            };

            if missing_ok && !std::path::Path::new(&path).exists() {
                return Ok(PyValue::None);
            }

            std::fs::remove_file(&path)
                .map(|_| PyValue::None)
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot remove file '{}': {}", path, e),
                })
        }),
        // Path.rename(target)
        PyBuiltinFunction::new("Path_rename", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let target = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::Dict(d)) => {
                    match d.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                        PyValue::Str(s) => s.to_string(),
                        _ => return Err(RuntimeError::type_error("str or Path", "invalid Path")),
                    }
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or Path",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::fs::rename(&path, &target)
                .map(|_| create_path_object(&target))
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot rename '{}' to '{}': {}", path, target, e),
                })
        }),
        // Path.read_text(encoding='utf-8')
        PyBuiltinFunction::new("Path_read_text", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            std::fs::read_to_string(&path).map(|s| PyValue::Str(Arc::from(s))).map_err(|e| {
                RuntimeError::OsError {
                    message: format!("Cannot read file '{}': {}", path, e),
                }
            })
        }),
        // Path.read_bytes()
        PyBuiltinFunction::new("Path_read_bytes", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let bytes = std::fs::read(&path).map_err(|e| RuntimeError::OsError {
                message: format!("Cannot read file '{}': {}", path, e),
            })?;

            let byte_values: Vec<PyValue> =
                bytes.into_iter().map(|b| PyValue::Int(b as i64)).collect();

            Ok(PyValue::List(Arc::new(crate::PyList::from_values(byte_values))))
        }),
        // Path.write_text(data, encoding='utf-8')
        PyBuiltinFunction::new("Path_write_text", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let data = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::fs::write(&path, &data)
                .map(|_| PyValue::Int(data.len() as i64))
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot write file '{}': {}", path, e),
                })
        }),
        // Path.write_bytes(data)
        PyBuiltinFunction::new("Path_write_bytes", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let data = match args.get(1) {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Int(i) = v {
                            Some(i as u8)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<u8>>(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "bytes",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            std::fs::write(&path, &data)
                .map(|_| PyValue::Int(data.len() as i64))
                .map_err(|e| RuntimeError::OsError {
                    message: format!("Cannot write file '{}': {}", path, e),
                })
        }),
        // Path.stat()
        PyBuiltinFunction::new("Path_stat", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let metadata = std::fs::metadata(&path).map_err(|e| RuntimeError::OsError {
                message: format!("Cannot stat '{}': {}", path, e),
            })?;

            let stat_dict = PyDict::new();
            stat_dict
                .setitem(PyKey::Str(Arc::from("st_size")), PyValue::Int(metadata.len() as i64));
            stat_dict.setitem(
                PyKey::Str(Arc::from("st_mode")),
                PyValue::Int(if metadata.is_dir() { 0o40755 } else { 0o100644 }),
            );

            let mtime = metadata
                .modified()
                .map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64
                })
                .unwrap_or(0);
            stat_dict.setitem(PyKey::Str(Arc::from("st_mtime")), PyValue::Int(mtime));

            Ok(PyValue::Dict(Arc::new(stat_dict)))
        }),
        // Path.__str__()
        PyBuiltinFunction::new("Path_str", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => Ok(PyValue::Str(s)),
                _ => Err(RuntimeError::internal_error("Invalid Path object")),
            }
        }),
        // Path.with_suffix(suffix)
        PyBuiltinFunction::new("Path_with_suffix", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let suffix = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let p = std::path::Path::new(&path);
            let new_path = p.with_extension(suffix.trim_start_matches('.'));

            Ok(create_path_object(&new_path.to_string_lossy()))
        }),
        // Path.with_name(name)
        PyBuiltinFunction::new("Path_with_name", |args| {
            let path_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "Path",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let path = match path_obj.get(&PyKey::Str(Arc::from("_path")), PyValue::None) {
                PyValue::Str(s) => s.to_string(),
                _ => return Err(RuntimeError::internal_error("Invalid Path object")),
            };

            let name = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let p = std::path::Path::new(&path);
            let new_path = p.with_file_name(&name);

            Ok(create_path_object(&new_path.to_string_lossy()))
        }),
    ]
}

// ===== asyncio module (Task 12.1) =====

/// Get the asyncio module as a dict
pub fn asyncio_module() -> Arc<PyDict> {
    let dict = PyDict::new();

    // Module info
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("asyncio")));

    // Constants
    dict.setitem(
        PyKey::Str(Arc::from("FIRST_COMPLETED")),
        PyValue::Str(Arc::from("FIRST_COMPLETED")),
    );
    dict.setitem(
        PyKey::Str(Arc::from("FIRST_EXCEPTION")),
        PyValue::Str(Arc::from("FIRST_EXCEPTION")),
    );
    dict.setitem(PyKey::Str(Arc::from("ALL_COMPLETED")), PyValue::Str(Arc::from("ALL_COMPLETED")));

    Arc::new(dict)
}

/// Create asyncio module builtins
pub fn asyncio_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // get_event_loop - get the current event loop
        PyBuiltinFunction::new("get_event_loop", |_args| Ok(create_event_loop())),
        // new_event_loop - create a new event loop
        PyBuiltinFunction::new("new_event_loop", |_args| Ok(create_event_loop())),
        // set_event_loop - set the current event loop
        PyBuiltinFunction::new("set_event_loop", |args| match args.first() {
            Some(PyValue::Dict(_)) | Some(PyValue::None) => Ok(PyValue::None),
            _ => Err(RuntimeError::type_error(
                "event loop or None",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // run - run a coroutine
        PyBuiltinFunction::new("run", |args| {
            match args.first() {
                Some(PyValue::Coroutine(coro)) => {
                    // Simplified: just return None for now
                    // Real implementation would execute the coroutine
                    let _ = coro;
                    Ok(PyValue::None)
                }
                Some(v) => Err(RuntimeError::type_error("coroutine", v.type_name())),
                None => Err(RuntimeError::type_error("coroutine", "nothing")),
            }
        }),
        // sleep - sleep for a number of seconds
        PyBuiltinFunction::new("sleep", |args| {
            let seconds = match args.first() {
                Some(PyValue::Int(n)) => *n as f64,
                Some(PyValue::Float(f)) => *f,
                _ => {
                    return Err(RuntimeError::type_error(
                        "number",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            // Create a Future that represents the sleep
            Ok(create_future_with_result(PyValue::None, seconds))
        }),
        // create_task - create a task from a coroutine
        PyBuiltinFunction::new("create_task", |args| match args.first() {
            Some(PyValue::Coroutine(coro)) => Ok(create_task(PyValue::Coroutine(Arc::clone(coro)))),
            Some(v) => Err(RuntimeError::type_error("coroutine", v.type_name())),
            None => Err(RuntimeError::type_error("coroutine", "nothing")),
        }),
        // gather - run multiple coroutines concurrently
        PyBuiltinFunction::new("gather", |args| {
            let tasks: Vec<PyValue> = args.iter().cloned().collect();
            Ok(create_gather_future(tasks))
        }),
        // wait - wait for futures with timeout
        PyBuiltinFunction::new("wait", |args| {
            let futures = match args.first() {
                Some(PyValue::List(l)) => l.to_vec(),
                Some(PyValue::Tuple(t)) => t.to_vec(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "iterable of futures",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            Ok(create_wait_result(futures))
        }),
        // wait_for - wait for a future with timeout
        PyBuiltinFunction::new("wait_for", |args| {
            let future = match args.first() {
                Some(v) => v.clone(),
                None => return Err(RuntimeError::type_error("awaitable", "nothing")),
            };
            let _timeout = match args.get(1) {
                Some(PyValue::Int(n)) => *n as f64,
                Some(PyValue::Float(f)) => *f,
                Some(PyValue::None) => f64::INFINITY,
                _ => return Err(RuntimeError::type_error("number or None", "other")),
            };
            Ok(future)
        }),
        // ensure_future - wrap a coroutine in a Future
        PyBuiltinFunction::new("ensure_future", |args| match args.first() {
            Some(PyValue::Coroutine(coro)) => Ok(create_task(PyValue::Coroutine(Arc::clone(coro)))),
            Some(PyValue::Dict(d)) if is_future(d) => Ok(PyValue::Dict(Arc::clone(d))),
            Some(v) => Err(RuntimeError::type_error("awaitable", v.type_name())),
            None => Err(RuntimeError::type_error("awaitable", "nothing")),
        }),
        // shield - protect a future from cancellation
        PyBuiltinFunction::new("shield", |args| match args.first() {
            Some(v) => Ok(v.clone()),
            None => Err(RuntimeError::type_error("awaitable", "nothing")),
        }),
        // iscoroutine - check if object is a coroutine
        PyBuiltinFunction::new("iscoroutine", |args| {
            Ok(PyValue::Bool(matches!(args.first(), Some(PyValue::Coroutine(_)))))
        }),
        // iscoroutinefunction - check if object is a coroutine function
        PyBuiltinFunction::new("iscoroutinefunction", |args| {
            // Simplified: check if it's a function with is_coroutine flag
            Ok(PyValue::Bool(false))
        }),
        // isfuture - check if object is a Future
        PyBuiltinFunction::new("isfuture", |args| match args.first() {
            Some(PyValue::Dict(d)) => Ok(PyValue::Bool(is_future(d))),
            _ => Ok(PyValue::Bool(false)),
        }),
        // current_task - get the currently running task
        PyBuiltinFunction::new("current_task", |_args| Ok(PyValue::None)),
        // all_tasks - get all tasks for an event loop
        PyBuiltinFunction::new("all_tasks", |_args| Ok(PyValue::List(Arc::new(PyList::new())))),
    ]
}

/// Create an event loop object
fn create_event_loop() -> PyValue {
    let loop_dict = PyDict::new();
    loop_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("EventLoop")));
    loop_dict.setitem(PyKey::Str(Arc::from("_running")), PyValue::Bool(false));
    loop_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));
    loop_dict.setitem(PyKey::Str(Arc::from("_tasks")), PyValue::List(Arc::new(PyList::new())));
    PyValue::Dict(Arc::new(loop_dict))
}

/// Create a Future object
fn create_future() -> PyValue {
    let future_dict = PyDict::new();
    future_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Future")));
    future_dict.setitem(PyKey::Str(Arc::from("_state")), PyValue::Str(Arc::from("PENDING")));
    future_dict.setitem(PyKey::Str(Arc::from("_result")), PyValue::None);
    future_dict.setitem(PyKey::Str(Arc::from("_exception")), PyValue::None);
    future_dict
        .setitem(PyKey::Str(Arc::from("_callbacks")), PyValue::List(Arc::new(PyList::new())));
    PyValue::Dict(Arc::new(future_dict))
}

/// Create a Future with a result
fn create_future_with_result(result: PyValue, _delay: f64) -> PyValue {
    let future_dict = PyDict::new();
    future_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Future")));
    future_dict.setitem(PyKey::Str(Arc::from("_state")), PyValue::Str(Arc::from("FINISHED")));
    future_dict.setitem(PyKey::Str(Arc::from("_result")), result);
    future_dict.setitem(PyKey::Str(Arc::from("_exception")), PyValue::None);
    future_dict
        .setitem(PyKey::Str(Arc::from("_callbacks")), PyValue::List(Arc::new(PyList::new())));
    PyValue::Dict(Arc::new(future_dict))
}

/// Create a Task object
fn create_task(coro: PyValue) -> PyValue {
    let task_dict = PyDict::new();
    task_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Task")));
    task_dict.setitem(PyKey::Str(Arc::from("_state")), PyValue::Str(Arc::from("PENDING")));
    task_dict.setitem(PyKey::Str(Arc::from("_coro")), coro);
    task_dict.setitem(PyKey::Str(Arc::from("_result")), PyValue::None);
    task_dict.setitem(PyKey::Str(Arc::from("_exception")), PyValue::None);
    task_dict.setitem(PyKey::Str(Arc::from("_callbacks")), PyValue::List(Arc::new(PyList::new())));
    PyValue::Dict(Arc::new(task_dict))
}

/// Create a gather Future
fn create_gather_future(tasks: Vec<PyValue>) -> PyValue {
    let future_dict = PyDict::new();
    future_dict
        .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("_GatheringFuture")));
    future_dict.setitem(PyKey::Str(Arc::from("_state")), PyValue::Str(Arc::from("PENDING")));
    future_dict.setitem(
        PyKey::Str(Arc::from("_children")),
        PyValue::List(Arc::new(PyList::from_values(tasks))),
    );
    future_dict.setitem(PyKey::Str(Arc::from("_result")), PyValue::None);
    future_dict.setitem(PyKey::Str(Arc::from("_exception")), PyValue::None);
    PyValue::Dict(Arc::new(future_dict))
}

/// Create a wait result (done, pending sets)
fn create_wait_result(futures: Vec<PyValue>) -> PyValue {
    let done = PyList::from_values(futures.clone());
    let pending = PyList::new();
    PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
        PyValue::List(Arc::new(done)),
        PyValue::List(Arc::new(pending)),
    ])))
}

/// Check if a dict is a Future
fn is_future(d: &Arc<PyDict>) -> bool {
    match d.get(&PyKey::Str(Arc::from("__class__")), PyValue::None) {
        PyValue::Str(s) => {
            s.as_ref() == "Future" || s.as_ref() == "Task" || s.as_ref() == "_GatheringFuture"
        }
        _ => false,
    }
}

/// Create asyncio event loop builtins
pub fn asyncio_loop_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // EventLoop.run_until_complete
        PyBuiltinFunction::new("EventLoop_run_until_complete", |args| {
            let _loop_obj = match args.first() {
                Some(PyValue::Dict(d)) => d,
                _ => {
                    return Err(RuntimeError::type_error(
                        "EventLoop",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            match args.get(1) {
                Some(PyValue::Coroutine(_)) | Some(PyValue::Dict(_)) => Ok(PyValue::None),
                _ => Err(RuntimeError::type_error(
                    "awaitable",
                    args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
                )),
            }
        }),
        // EventLoop.run_forever
        PyBuiltinFunction::new("EventLoop_run_forever", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_running")), PyValue::Bool(true));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "EventLoop",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // EventLoop.stop
        PyBuiltinFunction::new("EventLoop_stop", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_running")), PyValue::Bool(false));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "EventLoop",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // EventLoop.close
        PyBuiltinFunction::new("EventLoop_close", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(true));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "EventLoop",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // EventLoop.is_running
        PyBuiltinFunction::new("EventLoop_is_running", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                Ok(d.get(&PyKey::Str(Arc::from("_running")), PyValue::Bool(false)))
            }
            _ => Err(RuntimeError::type_error(
                "EventLoop",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // EventLoop.is_closed
        PyBuiltinFunction::new("EventLoop_is_closed", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                Ok(d.get(&PyKey::Str(Arc::from("_closed")), PyValue::Bool(false)))
            }
            _ => Err(RuntimeError::type_error(
                "EventLoop",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // EventLoop.create_future
        PyBuiltinFunction::new("EventLoop_create_future", |_args| Ok(create_future())),
        // EventLoop.create_task
        PyBuiltinFunction::new("EventLoop_create_task", |args| match args.get(1) {
            Some(PyValue::Coroutine(coro)) => Ok(create_task(PyValue::Coroutine(Arc::clone(coro)))),
            _ => Err(RuntimeError::type_error(
                "coroutine",
                args.get(1).map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
    ]
}

// ===== threading module (Task 12.2) =====

/// Get the threading module as a dict
pub fn threading_module() -> Arc<PyDict> {
    let dict = PyDict::new();
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("threading")));
    dict.setitem(PyKey::Str(Arc::from("TIMEOUT_MAX")), PyValue::Float(f64::MAX));
    Arc::new(dict)
}

/// Create threading module builtins
pub fn threading_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // Thread - create a new thread
        PyBuiltinFunction::new("Thread", |args| {
            let target = args.first().cloned().unwrap_or(PyValue::None);
            let name = match args.get(1) {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => format!("Thread-{}", std::process::id()),
            };
            let thread_dict = PyDict::new();
            thread_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Thread")));
            thread_dict.setitem(PyKey::Str(Arc::from("_target")), target);
            thread_dict.setitem(PyKey::Str(Arc::from("_name")), PyValue::Str(Arc::from(name)));
            thread_dict.setitem(PyKey::Str(Arc::from("_started")), PyValue::Bool(false));
            thread_dict.setitem(PyKey::Str(Arc::from("_alive")), PyValue::Bool(false));
            thread_dict.setitem(PyKey::Str(Arc::from("_daemon")), PyValue::Bool(false));
            thread_dict.setitem(PyKey::Str(Arc::from("_ident")), PyValue::None);
            Ok(PyValue::Dict(Arc::new(thread_dict)))
        }),
        // Lock - create a lock
        PyBuiltinFunction::new("Lock", |_args| {
            let lock_dict = PyDict::new();
            lock_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Lock")));
            lock_dict.setitem(PyKey::Str(Arc::from("_locked")), PyValue::Bool(false));
            Ok(PyValue::Dict(Arc::new(lock_dict)))
        }),
        // RLock - create a reentrant lock
        PyBuiltinFunction::new("RLock", |_args| {
            let lock_dict = PyDict::new();
            lock_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("RLock")));
            lock_dict.setitem(PyKey::Str(Arc::from("_locked")), PyValue::Bool(false));
            lock_dict.setitem(PyKey::Str(Arc::from("_count")), PyValue::Int(0));
            lock_dict.setitem(PyKey::Str(Arc::from("_owner")), PyValue::None);
            Ok(PyValue::Dict(Arc::new(lock_dict)))
        }),
        // Event - create an event
        PyBuiltinFunction::new("Event", |_args| {
            let event_dict = PyDict::new();
            event_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Event")));
            event_dict.setitem(PyKey::Str(Arc::from("_flag")), PyValue::Bool(false));
            Ok(PyValue::Dict(Arc::new(event_dict)))
        }),
        // Condition - create a condition variable
        PyBuiltinFunction::new("Condition", |args| {
            let lock = match args.first() {
                Some(PyValue::Dict(d)) => PyValue::Dict(Arc::clone(d)),
                _ => {
                    let lock_dict = PyDict::new();
                    lock_dict.setitem(
                        PyKey::Str(Arc::from("__class__")),
                        PyValue::Str(Arc::from("RLock")),
                    );
                    lock_dict.setitem(PyKey::Str(Arc::from("_locked")), PyValue::Bool(false));
                    lock_dict.setitem(PyKey::Str(Arc::from("_count")), PyValue::Int(0));
                    PyValue::Dict(Arc::new(lock_dict))
                }
            };
            let cond_dict = PyDict::new();
            cond_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Condition")));
            cond_dict.setitem(PyKey::Str(Arc::from("_lock")), lock);
            cond_dict
                .setitem(PyKey::Str(Arc::from("_waiters")), PyValue::List(Arc::new(PyList::new())));
            Ok(PyValue::Dict(Arc::new(cond_dict)))
        }),
        // Semaphore - create a semaphore
        PyBuiltinFunction::new("Semaphore", |args| {
            let value = match args.first() {
                Some(PyValue::Int(n)) if *n >= 0 => *n,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("semaphore initial value must be >= 0"))
                }
                None => 1,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };
            let sem_dict = PyDict::new();
            sem_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Semaphore")));
            sem_dict.setitem(PyKey::Str(Arc::from("_value")), PyValue::Int(value));
            Ok(PyValue::Dict(Arc::new(sem_dict)))
        }),
        // BoundedSemaphore - create a bounded semaphore
        PyBuiltinFunction::new("BoundedSemaphore", |args| {
            let value = match args.first() {
                Some(PyValue::Int(n)) if *n >= 0 => *n,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("semaphore initial value must be >= 0"))
                }
                None => 1,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };
            let sem_dict = PyDict::new();
            sem_dict.setitem(
                PyKey::Str(Arc::from("__class__")),
                PyValue::Str(Arc::from("BoundedSemaphore")),
            );
            sem_dict.setitem(PyKey::Str(Arc::from("_value")), PyValue::Int(value));
            sem_dict.setitem(PyKey::Str(Arc::from("_initial_value")), PyValue::Int(value));
            Ok(PyValue::Dict(Arc::new(sem_dict)))
        }),
        // Barrier - create a barrier
        PyBuiltinFunction::new("Barrier", |args| {
            let parties = match args.first() {
                Some(PyValue::Int(n)) if *n > 0 => *n,
                Some(PyValue::Int(_)) => {
                    return Err(RuntimeError::value_error("number of parties must be > 0"))
                }
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let barrier_dict = PyDict::new();
            barrier_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Barrier")));
            barrier_dict.setitem(PyKey::Str(Arc::from("_parties")), PyValue::Int(parties));
            barrier_dict.setitem(PyKey::Str(Arc::from("_count")), PyValue::Int(0));
            barrier_dict.setitem(PyKey::Str(Arc::from("_broken")), PyValue::Bool(false));
            Ok(PyValue::Dict(Arc::new(barrier_dict)))
        }),
        // Timer - create a timer thread
        PyBuiltinFunction::new("Timer", |args| {
            let interval = match args.first() {
                Some(PyValue::Int(n)) => *n as f64,
                Some(PyValue::Float(f)) => *f,
                _ => {
                    return Err(RuntimeError::type_error(
                        "number",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let function = args.get(1).cloned().unwrap_or(PyValue::None);
            let timer_dict = PyDict::new();
            timer_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Timer")));
            timer_dict.setitem(PyKey::Str(Arc::from("_interval")), PyValue::Float(interval));
            timer_dict.setitem(PyKey::Str(Arc::from("_function")), function);
            timer_dict.setitem(PyKey::Str(Arc::from("_started")), PyValue::Bool(false));
            timer_dict.setitem(PyKey::Str(Arc::from("_cancelled")), PyValue::Bool(false));
            Ok(PyValue::Dict(Arc::new(timer_dict)))
        }),
        // current_thread - get the current thread
        PyBuiltinFunction::new("current_thread", |_args| {
            let thread_dict = PyDict::new();
            thread_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Thread")));
            thread_dict
                .setitem(PyKey::Str(Arc::from("_name")), PyValue::Str(Arc::from("MainThread")));
            thread_dict.setitem(PyKey::Str(Arc::from("_started")), PyValue::Bool(true));
            thread_dict.setitem(PyKey::Str(Arc::from("_alive")), PyValue::Bool(true));
            thread_dict.setitem(PyKey::Str(Arc::from("_daemon")), PyValue::Bool(false));
            thread_dict
                .setitem(PyKey::Str(Arc::from("_ident")), PyValue::Int(std::process::id() as i64));
            Ok(PyValue::Dict(Arc::new(thread_dict)))
        }),
        // main_thread - get the main thread
        PyBuiltinFunction::new("main_thread", |_args| {
            let thread_dict = PyDict::new();
            thread_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Thread")));
            thread_dict
                .setitem(PyKey::Str(Arc::from("_name")), PyValue::Str(Arc::from("MainThread")));
            thread_dict.setitem(PyKey::Str(Arc::from("_started")), PyValue::Bool(true));
            thread_dict.setitem(PyKey::Str(Arc::from("_alive")), PyValue::Bool(true));
            thread_dict.setitem(PyKey::Str(Arc::from("_daemon")), PyValue::Bool(false));
            Ok(PyValue::Dict(Arc::new(thread_dict)))
        }),
        // active_count - get number of active threads
        PyBuiltinFunction::new("active_count", |_args| {
            Ok(PyValue::Int(1)) // Main thread
        }),
        // enumerate - list all active threads
        PyBuiltinFunction::new("enumerate", |_args| {
            let main_thread = PyDict::new();
            main_thread
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Thread")));
            main_thread
                .setitem(PyKey::Str(Arc::from("_name")), PyValue::Str(Arc::from("MainThread")));
            main_thread.setitem(PyKey::Str(Arc::from("_alive")), PyValue::Bool(true));
            Ok(PyValue::List(Arc::new(PyList::from_values(vec![PyValue::Dict(Arc::new(
                main_thread,
            ))]))))
        }),
        // get_ident - get current thread identifier
        PyBuiltinFunction::new("get_ident", |_args| Ok(PyValue::Int(std::process::id() as i64))),
        // get_native_id - get native thread ID
        PyBuiltinFunction::new("get_native_id", |_args| {
            Ok(PyValue::Int(std::process::id() as i64))
        }),
        // stack_size - get/set thread stack size
        PyBuiltinFunction::new("stack_size", |args| match args.first() {
            Some(PyValue::Int(size)) if *size >= 0 => Ok(PyValue::Int(0)),
            Some(PyValue::Int(_)) => Err(RuntimeError::value_error("size must be >= 0")),
            None => Ok(PyValue::Int(0)),
            Some(v) => Err(RuntimeError::type_error("int", v.type_name())),
        }),
        // settrace - set trace function for all threads
        PyBuiltinFunction::new("settrace", |_args| Ok(PyValue::None)),
        // setprofile - set profile function for all threads
        PyBuiltinFunction::new("setprofile", |_args| Ok(PyValue::None)),
    ]
}

/// Create threading object method builtins
pub fn threading_object_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // Thread.start
        PyBuiltinFunction::new("Thread_start", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_started")), PyValue::Bool(true));
                d.setitem(PyKey::Str(Arc::from("_alive")), PyValue::Bool(true));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Thread",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Thread.join
        PyBuiltinFunction::new("Thread_join", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_alive")), PyValue::Bool(false));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Thread",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Thread.is_alive
        PyBuiltinFunction::new("Thread_is_alive", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                Ok(d.get(&PyKey::Str(Arc::from("_alive")), PyValue::Bool(false)))
            }
            _ => Err(RuntimeError::type_error(
                "Thread",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Lock.acquire
        PyBuiltinFunction::new("Lock_acquire", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_locked")), PyValue::Bool(true));
                Ok(PyValue::Bool(true))
            }
            _ => Err(RuntimeError::type_error(
                "Lock",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Lock.release
        PyBuiltinFunction::new("Lock_release", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_locked")), PyValue::Bool(false));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Lock",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Lock.locked
        PyBuiltinFunction::new("Lock_locked", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                Ok(d.get(&PyKey::Str(Arc::from("_locked")), PyValue::Bool(false)))
            }
            _ => Err(RuntimeError::type_error(
                "Lock",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Event.set
        PyBuiltinFunction::new("Event_set", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_flag")), PyValue::Bool(true));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Event",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Event.clear
        PyBuiltinFunction::new("Event_clear", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_flag")), PyValue::Bool(false));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Event",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Event.is_set
        PyBuiltinFunction::new("Event_is_set", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                Ok(d.get(&PyKey::Str(Arc::from("_flag")), PyValue::Bool(false)))
            }
            _ => Err(RuntimeError::type_error(
                "Event",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Event.wait
        PyBuiltinFunction::new("Event_wait", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                Ok(d.get(&PyKey::Str(Arc::from("_flag")), PyValue::Bool(false)))
            }
            _ => Err(RuntimeError::type_error(
                "Event",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Semaphore.acquire
        PyBuiltinFunction::new("Semaphore_acquire", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                if let PyValue::Int(v) = d.get(&PyKey::Str(Arc::from("_value")), PyValue::Int(0)) {
                    if v > 0 {
                        d.setitem(PyKey::Str(Arc::from("_value")), PyValue::Int(v - 1));
                        Ok(PyValue::Bool(true))
                    } else {
                        Ok(PyValue::Bool(false))
                    }
                } else {
                    Ok(PyValue::Bool(false))
                }
            }
            _ => Err(RuntimeError::type_error(
                "Semaphore",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Semaphore.release
        PyBuiltinFunction::new("Semaphore_release", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                if let PyValue::Int(v) = d.get(&PyKey::Str(Arc::from("_value")), PyValue::Int(0)) {
                    d.setitem(PyKey::Str(Arc::from("_value")), PyValue::Int(v + 1));
                }
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Semaphore",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
    ]
}

// ===== subprocess module (Task 12.3) =====

/// Get the subprocess module as a dict
pub fn subprocess_module() -> Arc<PyDict> {
    let dict = PyDict::new();
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("subprocess")));
    // Constants
    dict.setitem(PyKey::Str(Arc::from("PIPE")), PyValue::Int(-1));
    dict.setitem(PyKey::Str(Arc::from("STDOUT")), PyValue::Int(-2));
    dict.setitem(PyKey::Str(Arc::from("DEVNULL")), PyValue::Int(-3));
    Arc::new(dict)
}

/// Create subprocess module builtins
pub fn subprocess_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // Popen - create a subprocess
        PyBuiltinFunction::new("Popen", |args| {
            let cmd_args: Vec<String> = match args.first() {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Str(s) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Some(PyValue::Str(s)) => vec![s.to_string()],
                _ => {
                    return Err(RuntimeError::type_error(
                        "list or str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let popen_dict = PyDict::new();
            popen_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Popen")));
            popen_dict.setitem(
                PyKey::Str(Arc::from("args")),
                PyValue::List(Arc::new(PyList::from_values(
                    cmd_args.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect(),
                ))),
            );
            popen_dict.setitem(PyKey::Str(Arc::from("returncode")), PyValue::None);
            popen_dict.setitem(PyKey::Str(Arc::from("pid")), PyValue::Int(0));
            popen_dict.setitem(PyKey::Str(Arc::from("stdin")), PyValue::None);
            popen_dict.setitem(PyKey::Str(Arc::from("stdout")), PyValue::None);
            popen_dict.setitem(PyKey::Str(Arc::from("stderr")), PyValue::None);
            Ok(PyValue::Dict(Arc::new(popen_dict)))
        }),
        // run - run a command and wait for completion
        PyBuiltinFunction::new("run", |args| {
            let cmd_args: Vec<String> = match args.first() {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Str(s) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Some(PyValue::Str(s)) => vec![s.to_string()],
                _ => {
                    return Err(RuntimeError::type_error(
                        "list or str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            if cmd_args.is_empty() {
                return Err(RuntimeError::value_error("empty command"));
            }

            // Execute the command
            let output = std::process::Command::new(&cmd_args[0]).args(&cmd_args[1..]).output();

            match output {
                Ok(out) => {
                    let result_dict = PyDict::new();
                    result_dict.setitem(
                        PyKey::Str(Arc::from("__class__")),
                        PyValue::Str(Arc::from("CompletedProcess")),
                    );
                    result_dict.setitem(
                        PyKey::Str(Arc::from("args")),
                        PyValue::List(Arc::new(PyList::from_values(
                            cmd_args.iter().map(|s| PyValue::Str(Arc::from(s.clone()))).collect(),
                        ))),
                    );
                    result_dict.setitem(
                        PyKey::Str(Arc::from("returncode")),
                        PyValue::Int(out.status.code().unwrap_or(-1) as i64),
                    );
                    result_dict.setitem(
                        PyKey::Str(Arc::from("stdout")),
                        PyValue::Str(Arc::from(String::from_utf8_lossy(&out.stdout).to_string())),
                    );
                    result_dict.setitem(
                        PyKey::Str(Arc::from("stderr")),
                        PyValue::Str(Arc::from(String::from_utf8_lossy(&out.stderr).to_string())),
                    );
                    Ok(PyValue::Dict(Arc::new(result_dict)))
                }
                Err(e) => Err(RuntimeError::OsError {
                    message: format!("Failed to execute command: {}", e),
                }),
            }
        }),
        // call - run a command and return the return code
        PyBuiltinFunction::new("call", |args| {
            let cmd_args: Vec<String> = match args.first() {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Str(s) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Some(PyValue::Str(s)) => vec![s.to_string()],
                _ => {
                    return Err(RuntimeError::type_error(
                        "list or str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            if cmd_args.is_empty() {
                return Err(RuntimeError::value_error("empty command"));
            }

            let status = std::process::Command::new(&cmd_args[0]).args(&cmd_args[1..]).status();

            match status {
                Ok(s) => Ok(PyValue::Int(s.code().unwrap_or(-1) as i64)),
                Err(e) => Err(RuntimeError::OsError {
                    message: format!("Failed to execute command: {}", e),
                }),
            }
        }),
        // check_call - run a command and raise on non-zero return
        PyBuiltinFunction::new("check_call", |args| {
            let cmd_args: Vec<String> = match args.first() {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Str(s) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Some(PyValue::Str(s)) => vec![s.to_string()],
                _ => {
                    return Err(RuntimeError::type_error(
                        "list or str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            if cmd_args.is_empty() {
                return Err(RuntimeError::value_error("empty command"));
            }

            let status = std::process::Command::new(&cmd_args[0]).args(&cmd_args[1..]).status();

            match status {
                Ok(s) => {
                    let code = s.code().unwrap_or(-1);
                    if code != 0 {
                        Err(RuntimeError::internal_error(format!(
                            "Command returned non-zero exit status {}",
                            code
                        )))
                    } else {
                        Ok(PyValue::Int(0))
                    }
                }
                Err(e) => Err(RuntimeError::OsError {
                    message: format!("Failed to execute command: {}", e),
                }),
            }
        }),
        // check_output - run a command and return its output
        PyBuiltinFunction::new("check_output", |args| {
            let cmd_args: Vec<String> = match args.first() {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Str(s) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
                Some(PyValue::Str(s)) => vec![s.to_string()],
                _ => {
                    return Err(RuntimeError::type_error(
                        "list or str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            if cmd_args.is_empty() {
                return Err(RuntimeError::value_error("empty command"));
            }

            let output = std::process::Command::new(&cmd_args[0]).args(&cmd_args[1..]).output();

            match output {
                Ok(out) => {
                    if !out.status.success() {
                        Err(RuntimeError::internal_error(format!(
                            "Command returned non-zero exit status {}",
                            out.status.code().unwrap_or(-1)
                        )))
                    } else {
                        Ok(PyValue::Str(Arc::from(
                            String::from_utf8_lossy(&out.stdout).to_string(),
                        )))
                    }
                }
                Err(e) => Err(RuntimeError::OsError {
                    message: format!("Failed to execute command: {}", e),
                }),
            }
        }),
        // getoutput - run a command through the shell and return output
        PyBuiltinFunction::new("getoutput", |args| {
            let cmd = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };

            let shell = if cfg!(target_os = "windows") {
                "cmd"
            } else {
                "sh"
            };
            let shell_arg = if cfg!(target_os = "windows") {
                "/C"
            } else {
                "-c"
            };

            let output = std::process::Command::new(shell).args([shell_arg, &cmd]).output();

            match output {
                Ok(out) => {
                    Ok(PyValue::Str(Arc::from(String::from_utf8_lossy(&out.stdout).to_string())))
                }
                Err(e) => Err(RuntimeError::OsError {
                    message: format!("Failed to execute command: {}", e),
                }),
            }
        }),
    ]
}

/// Create subprocess Popen method builtins
pub fn subprocess_popen_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // Popen.poll
        PyBuiltinFunction::new("Popen_poll", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                Ok(d.get(&PyKey::Str(Arc::from("returncode")), PyValue::None))
            }
            _ => Err(RuntimeError::type_error(
                "Popen",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Popen.wait
        PyBuiltinFunction::new("Popen_wait", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("returncode")), PyValue::Int(0));
                Ok(PyValue::Int(0))
            }
            _ => Err(RuntimeError::type_error(
                "Popen",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Popen.communicate
        PyBuiltinFunction::new("Popen_communicate", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("returncode")), PyValue::Int(0));
                Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                    PyValue::Str(Arc::from("")),
                    PyValue::Str(Arc::from("")),
                ]))))
            }
            _ => Err(RuntimeError::type_error(
                "Popen",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Popen.terminate
        PyBuiltinFunction::new("Popen_terminate", |args| match args.first() {
            Some(PyValue::Dict(_)) => Ok(PyValue::None),
            _ => Err(RuntimeError::type_error(
                "Popen",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Popen.kill
        PyBuiltinFunction::new("Popen_kill", |args| match args.first() {
            Some(PyValue::Dict(_)) => Ok(PyValue::None),
            _ => Err(RuntimeError::type_error(
                "Popen",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
    ]
}

// ===== socket module (Task 12.4) =====

/// Get the socket module as a dict
pub fn socket_module() -> Arc<PyDict> {
    let dict = PyDict::new();
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("socket")));
    // Address families
    dict.setitem(PyKey::Str(Arc::from("AF_INET")), PyValue::Int(2));
    dict.setitem(PyKey::Str(Arc::from("AF_INET6")), PyValue::Int(10));
    dict.setitem(PyKey::Str(Arc::from("AF_UNIX")), PyValue::Int(1));
    // Socket types
    dict.setitem(PyKey::Str(Arc::from("SOCK_STREAM")), PyValue::Int(1));
    dict.setitem(PyKey::Str(Arc::from("SOCK_DGRAM")), PyValue::Int(2));
    dict.setitem(PyKey::Str(Arc::from("SOCK_RAW")), PyValue::Int(3));
    // IP protocols
    dict.setitem(PyKey::Str(Arc::from("IPPROTO_TCP")), PyValue::Int(6));
    dict.setitem(PyKey::Str(Arc::from("IPPROTO_UDP")), PyValue::Int(17));
    // Socket options
    dict.setitem(PyKey::Str(Arc::from("SOL_SOCKET")), PyValue::Int(1));
    dict.setitem(PyKey::Str(Arc::from("SO_REUSEADDR")), PyValue::Int(2));
    dict.setitem(PyKey::Str(Arc::from("SO_KEEPALIVE")), PyValue::Int(9));
    dict.setitem(PyKey::Str(Arc::from("SO_BROADCAST")), PyValue::Int(6));
    // Shutdown modes
    dict.setitem(PyKey::Str(Arc::from("SHUT_RD")), PyValue::Int(0));
    dict.setitem(PyKey::Str(Arc::from("SHUT_WR")), PyValue::Int(1));
    dict.setitem(PyKey::Str(Arc::from("SHUT_RDWR")), PyValue::Int(2));
    Arc::new(dict)
}

/// Create socket module builtins
pub fn socket_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // socket - create a socket
        PyBuiltinFunction::new("socket", |args| {
            let family = match args.first() {
                Some(PyValue::Int(n)) => *n,
                None => 2, // AF_INET
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };
            let sock_type = match args.get(1) {
                Some(PyValue::Int(n)) => *n,
                None => 1, // SOCK_STREAM
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };
            let proto = match args.get(2) {
                Some(PyValue::Int(n)) => *n,
                None => 0,
                Some(v) => return Err(RuntimeError::type_error("int", v.type_name())),
            };

            let sock_dict = PyDict::new();
            sock_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("socket")));
            sock_dict.setitem(PyKey::Str(Arc::from("family")), PyValue::Int(family));
            sock_dict.setitem(PyKey::Str(Arc::from("type")), PyValue::Int(sock_type));
            sock_dict.setitem(PyKey::Str(Arc::from("proto")), PyValue::Int(proto));
            sock_dict.setitem(PyKey::Str(Arc::from("_fd")), PyValue::Int(-1));
            sock_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));
            sock_dict.setitem(PyKey::Str(Arc::from("_timeout")), PyValue::None);
            Ok(PyValue::Dict(Arc::new(sock_dict)))
        }),
        // gethostname - get the hostname
        PyBuiltinFunction::new("gethostname", |_args| match hostname::get() {
            Ok(name) => Ok(PyValue::Str(Arc::from(name.to_string_lossy().to_string()))),
            Err(_) => Ok(PyValue::Str(Arc::from("localhost"))),
        }),
        // gethostbyname - resolve hostname to IP
        PyBuiltinFunction::new("gethostbyname", |args| {
            let hostname = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            // Simplified: just return localhost for now
            if hostname == "localhost" || hostname == "127.0.0.1" {
                Ok(PyValue::Str(Arc::from("127.0.0.1")))
            } else {
                Ok(PyValue::Str(Arc::from(hostname)))
            }
        }),
        // getaddrinfo - resolve address info
        PyBuiltinFunction::new("getaddrinfo", |args| {
            let host = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::None) => "".to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str or None",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let port = match args.get(1) {
                Some(PyValue::Int(n)) => *n,
                Some(PyValue::Str(s)) => s.parse().unwrap_or(0),
                Some(PyValue::None) => 0,
                _ => return Err(RuntimeError::type_error("int, str, or None", "other")),
            };

            // Return a simplified result
            let result =
                PyList::from_values(vec![PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                    PyValue::Int(2), // AF_INET
                    PyValue::Int(1), // SOCK_STREAM
                    PyValue::Int(6), // IPPROTO_TCP
                    PyValue::Str(Arc::from("")),
                    PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                        PyValue::Str(Arc::from(if host.is_empty() { "0.0.0.0" } else { &host })),
                        PyValue::Int(port),
                    ]))),
                ])))]);
            Ok(PyValue::List(Arc::new(result)))
        }),
        // inet_aton - convert IP string to packed binary
        PyBuiltinFunction::new("inet_aton", |args| {
            let ip = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let parts: Vec<u8> = ip.split('.').filter_map(|p| p.parse().ok()).collect();
            if parts.len() != 4 {
                return Err(RuntimeError::value_error("illegal IP address string"));
            }
            Ok(PyValue::List(Arc::new(PyList::from_values(
                parts.into_iter().map(|b| PyValue::Int(b as i64)).collect(),
            ))))
        }),
        // inet_ntoa - convert packed binary to IP string
        PyBuiltinFunction::new("inet_ntoa", |args| {
            let bytes: Vec<u8> = match args.first() {
                Some(PyValue::List(l)) => l
                    .to_vec()
                    .into_iter()
                    .filter_map(|v| {
                        if let PyValue::Int(n) = v {
                            Some(n as u8)
                        } else {
                            None
                        }
                    })
                    .collect(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "bytes",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            if bytes.len() != 4 {
                return Err(RuntimeError::value_error("packed IP wrong length"));
            }
            Ok(PyValue::Str(Arc::from(format!(
                "{}.{}.{}.{}",
                bytes[0], bytes[1], bytes[2], bytes[3]
            ))))
        }),
        // htons - convert host to network byte order (short)
        PyBuiltinFunction::new("htons", |args| match args.first() {
            Some(PyValue::Int(n)) => Ok(PyValue::Int((*n as u16).to_be() as i64)),
            _ => Err(RuntimeError::type_error(
                "int",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // htonl - convert host to network byte order (long)
        PyBuiltinFunction::new("htonl", |args| match args.first() {
            Some(PyValue::Int(n)) => Ok(PyValue::Int((*n as u32).to_be() as i64)),
            _ => Err(RuntimeError::type_error(
                "int",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // ntohs - convert network to host byte order (short)
        PyBuiltinFunction::new("ntohs", |args| match args.first() {
            Some(PyValue::Int(n)) => Ok(PyValue::Int(u16::from_be(*n as u16) as i64)),
            _ => Err(RuntimeError::type_error(
                "int",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // ntohl - convert network to host byte order (long)
        PyBuiltinFunction::new("ntohl", |args| match args.first() {
            Some(PyValue::Int(n)) => Ok(PyValue::Int(u32::from_be(*n as u32) as i64)),
            _ => Err(RuntimeError::type_error(
                "int",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // setdefaulttimeout - set default socket timeout
        PyBuiltinFunction::new("setdefaulttimeout", |_args| Ok(PyValue::None)),
        // getdefaulttimeout - get default socket timeout
        PyBuiltinFunction::new("getdefaulttimeout", |_args| Ok(PyValue::None)),
        // create_connection - convenience function to create a connected socket
        PyBuiltinFunction::new("create_connection", |args| {
            let address = match args.first() {
                Some(PyValue::Tuple(t)) => t.to_vec(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "tuple",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            if address.len() < 2 {
                return Err(RuntimeError::value_error("address tuple must have host and port"));
            }

            let sock_dict = PyDict::new();
            sock_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("socket")));
            sock_dict.setitem(PyKey::Str(Arc::from("family")), PyValue::Int(2));
            sock_dict.setitem(PyKey::Str(Arc::from("type")), PyValue::Int(1));
            sock_dict.setitem(PyKey::Str(Arc::from("proto")), PyValue::Int(0));
            sock_dict.setitem(PyKey::Str(Arc::from("_fd")), PyValue::Int(-1));
            sock_dict.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(false));
            sock_dict.setitem(PyKey::Str(Arc::from("_connected")), PyValue::Bool(true));
            sock_dict.setitem(
                PyKey::Str(Arc::from("_address")),
                PyValue::Tuple(Arc::new(PyTuple::from_values(address))),
            );
            Ok(PyValue::Dict(Arc::new(sock_dict)))
        }),
    ]
}

/// Create socket object method builtins
pub fn socket_object_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // socket.bind
        PyBuiltinFunction::new("socket_bind", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                if let Some(PyValue::Tuple(addr)) = args.get(1) {
                    d.setitem(PyKey::Str(Arc::from("_bound")), PyValue::Bool(true));
                    d.setitem(PyKey::Str(Arc::from("_address")), PyValue::Tuple(Arc::clone(addr)));
                }
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.listen
        PyBuiltinFunction::new("socket_listen", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_listening")), PyValue::Bool(true));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.accept
        PyBuiltinFunction::new("socket_accept", |args| match args.first() {
            Some(PyValue::Dict(_)) => {
                let client_sock = PyDict::new();
                client_sock
                    .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("socket")));
                client_sock.setitem(PyKey::Str(Arc::from("family")), PyValue::Int(2));
                client_sock.setitem(PyKey::Str(Arc::from("type")), PyValue::Int(1));
                client_sock.setitem(PyKey::Str(Arc::from("_connected")), PyValue::Bool(true));
                let addr = PyTuple::from_values(vec![
                    PyValue::Str(Arc::from("127.0.0.1")),
                    PyValue::Int(0),
                ]);
                Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                    PyValue::Dict(Arc::new(client_sock)),
                    PyValue::Tuple(Arc::new(addr)),
                ]))))
            }
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.connect
        PyBuiltinFunction::new("socket_connect", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                if let Some(PyValue::Tuple(addr)) = args.get(1) {
                    d.setitem(PyKey::Str(Arc::from("_connected")), PyValue::Bool(true));
                    d.setitem(PyKey::Str(Arc::from("_address")), PyValue::Tuple(Arc::clone(addr)));
                }
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.send
        PyBuiltinFunction::new("socket_send", |args| {
            let data = match args.get(1) {
                Some(PyValue::Str(s)) => s.len(),
                Some(PyValue::List(l)) => l.len(),
                _ => 0,
            };
            Ok(PyValue::Int(data as i64))
        }),
        // socket.sendall
        PyBuiltinFunction::new("socket_sendall", |_args| Ok(PyValue::None)),
        // socket.recv
        PyBuiltinFunction::new("socket_recv", |args| {
            let bufsize = match args.get(1) {
                Some(PyValue::Int(n)) => *n as usize,
                _ => 1024,
            };
            // Return empty bytes (simplified)
            Ok(PyValue::List(Arc::new(PyList::from_values(vec![
                PyValue::Int(0);
                bufsize.min(0)
            ]))))
        }),
        // socket.recvfrom
        PyBuiltinFunction::new("socket_recvfrom", |args| {
            let bufsize = match args.get(1) {
                Some(PyValue::Int(n)) => *n as usize,
                _ => 1024,
            };
            let data = PyList::from_values(vec![PyValue::Int(0); bufsize.min(0)]);
            let addr =
                PyTuple::from_values(vec![PyValue::Str(Arc::from("127.0.0.1")), PyValue::Int(0)]);
            Ok(PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                PyValue::List(Arc::new(data)),
                PyValue::Tuple(Arc::new(addr)),
            ]))))
        }),
        // socket.sendto
        PyBuiltinFunction::new("socket_sendto", |args| {
            let data = match args.get(1) {
                Some(PyValue::Str(s)) => s.len(),
                Some(PyValue::List(l)) => l.len(),
                _ => 0,
            };
            Ok(PyValue::Int(data as i64))
        }),
        // socket.close
        PyBuiltinFunction::new("socket_close", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                d.setitem(PyKey::Str(Arc::from("_closed")), PyValue::Bool(true));
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.shutdown
        PyBuiltinFunction::new("socket_shutdown", |args| match args.first() {
            Some(PyValue::Dict(_)) => Ok(PyValue::None),
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.setsockopt
        PyBuiltinFunction::new("socket_setsockopt", |args| match args.first() {
            Some(PyValue::Dict(_)) => Ok(PyValue::None),
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.getsockopt
        PyBuiltinFunction::new("socket_getsockopt", |_args| Ok(PyValue::Int(0))),
        // socket.settimeout
        PyBuiltinFunction::new("socket_settimeout", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                let timeout = args.get(1).cloned().unwrap_or(PyValue::None);
                d.setitem(PyKey::Str(Arc::from("_timeout")), timeout);
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.gettimeout
        PyBuiltinFunction::new("socket_gettimeout", |args| match args.first() {
            Some(PyValue::Dict(d)) => Ok(d.get(&PyKey::Str(Arc::from("_timeout")), PyValue::None)),
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.fileno
        PyBuiltinFunction::new("socket_fileno", |args| match args.first() {
            Some(PyValue::Dict(d)) => Ok(d.get(&PyKey::Str(Arc::from("_fd")), PyValue::Int(-1))),
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.getpeername
        PyBuiltinFunction::new("socket_getpeername", |args| match args.first() {
            Some(PyValue::Dict(d)) => Ok(d.get(
                &PyKey::Str(Arc::from("_address")),
                PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                    PyValue::Str(Arc::from("")),
                    PyValue::Int(0),
                ]))),
            )),
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // socket.getsockname
        PyBuiltinFunction::new("socket_getsockname", |args| match args.first() {
            Some(PyValue::Dict(d)) => Ok(d.get(
                &PyKey::Str(Arc::from("_address")),
                PyValue::Tuple(Arc::new(PyTuple::from_values(vec![
                    PyValue::Str(Arc::from("0.0.0.0")),
                    PyValue::Int(0),
                ]))),
            )),
            _ => Err(RuntimeError::type_error(
                "socket",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
    ]
}

// ===== logging module (Task 12.5) =====

/// Get the logging module as a dict
pub fn logging_module() -> Arc<PyDict> {
    let dict = PyDict::new();
    dict.setitem(PyKey::Str(Arc::from("__name__")), PyValue::Str(Arc::from("logging")));
    // Log levels
    dict.setitem(PyKey::Str(Arc::from("CRITICAL")), PyValue::Int(50));
    dict.setitem(PyKey::Str(Arc::from("FATAL")), PyValue::Int(50));
    dict.setitem(PyKey::Str(Arc::from("ERROR")), PyValue::Int(40));
    dict.setitem(PyKey::Str(Arc::from("WARNING")), PyValue::Int(30));
    dict.setitem(PyKey::Str(Arc::from("WARN")), PyValue::Int(30));
    dict.setitem(PyKey::Str(Arc::from("INFO")), PyValue::Int(20));
    dict.setitem(PyKey::Str(Arc::from("DEBUG")), PyValue::Int(10));
    dict.setitem(PyKey::Str(Arc::from("NOTSET")), PyValue::Int(0));
    Arc::new(dict)
}

/// Create logging module builtins
pub fn logging_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // getLogger - get a logger by name
        PyBuiltinFunction::new("getLogger", |args| {
            let name = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                Some(PyValue::None) | None => "root".to_string(),
                Some(v) => return Err(RuntimeError::type_error("str or None", v.type_name())),
            };
            Ok(create_logger(&name))
        }),
        // basicConfig - configure the root logger
        PyBuiltinFunction::new("basicConfig", |_args| Ok(PyValue::None)),
        // debug - log at DEBUG level
        PyBuiltinFunction::new("debug", |args| log_message(10, args)),
        // info - log at INFO level
        PyBuiltinFunction::new("info", |args| log_message(20, args)),
        // warning - log at WARNING level
        PyBuiltinFunction::new("warning", |args| log_message(30, args)),
        // error - log at ERROR level
        PyBuiltinFunction::new("error", |args| log_message(40, args)),
        // critical - log at CRITICAL level
        PyBuiltinFunction::new("critical", |args| log_message(50, args)),
        // exception - log at ERROR level with exception info
        PyBuiltinFunction::new("exception", |args| log_message(40, args)),
        // log - log at specified level
        PyBuiltinFunction::new("log", |args| {
            let level = match args.first() {
                Some(PyValue::Int(n)) => *n,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            log_message(level, &args[1..])
        }),
        // setLevel - set the root logger level
        PyBuiltinFunction::new("setLevel", |_args| Ok(PyValue::None)),
        // disable - disable logging at or below a level
        PyBuiltinFunction::new("disable", |_args| Ok(PyValue::None)),
        // addLevelName - add a custom level name
        PyBuiltinFunction::new("addLevelName", |_args| Ok(PyValue::None)),
        // getLevelName - get the name for a level
        PyBuiltinFunction::new("getLevelName", |args| {
            let level = match args.first() {
                Some(PyValue::Int(n)) => *n,
                _ => {
                    return Err(RuntimeError::type_error(
                        "int",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let name = match level {
                50 => "CRITICAL",
                40 => "ERROR",
                30 => "WARNING",
                20 => "INFO",
                10 => "DEBUG",
                0 => "NOTSET",
                _ => "Level",
            };
            Ok(PyValue::Str(Arc::from(name)))
        }),
        // Logger - create a logger class
        PyBuiltinFunction::new("Logger", |args| {
            let name = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            Ok(create_logger(&name))
        }),
        // Handler - create a handler class
        PyBuiltinFunction::new("Handler", |_args| Ok(create_handler())),
        // StreamHandler - create a stream handler
        PyBuiltinFunction::new("StreamHandler", |_args| {
            let handler = create_handler();
            if let PyValue::Dict(d) = &handler {
                d.setitem(
                    PyKey::Str(Arc::from("__class__")),
                    PyValue::Str(Arc::from("StreamHandler")),
                );
            }
            Ok(handler)
        }),
        // FileHandler - create a file handler
        PyBuiltinFunction::new("FileHandler", |args| {
            let filename = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                _ => {
                    return Err(RuntimeError::type_error(
                        "str",
                        args.first().map(|v| v.type_name()).unwrap_or("nothing"),
                    ))
                }
            };
            let handler = create_handler();
            if let PyValue::Dict(d) = &handler {
                d.setitem(
                    PyKey::Str(Arc::from("__class__")),
                    PyValue::Str(Arc::from("FileHandler")),
                );
                d.setitem(PyKey::Str(Arc::from("_filename")), PyValue::Str(Arc::from(filename)));
            }
            Ok(handler)
        }),
        // Formatter - create a formatter
        PyBuiltinFunction::new("Formatter", |args| {
            let fmt = match args.first() {
                Some(PyValue::Str(s)) => s.to_string(),
                None => "%(message)s".to_string(),
                Some(v) => return Err(RuntimeError::type_error("str", v.type_name())),
            };
            let datefmt = match args.get(1) {
                Some(PyValue::Str(s)) => Some(s.to_string()),
                _ => None,
            };
            let formatter_dict = PyDict::new();
            formatter_dict
                .setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Formatter")));
            formatter_dict.setitem(PyKey::Str(Arc::from("_fmt")), PyValue::Str(Arc::from(fmt)));
            formatter_dict.setitem(
                PyKey::Str(Arc::from("_datefmt")),
                match datefmt {
                    Some(s) => PyValue::Str(Arc::from(s)),
                    None => PyValue::None,
                },
            );
            Ok(PyValue::Dict(Arc::new(formatter_dict)))
        }),
        // NullHandler - create a null handler
        PyBuiltinFunction::new("NullHandler", |_args| {
            let handler = create_handler();
            if let PyValue::Dict(d) = &handler {
                d.setitem(
                    PyKey::Str(Arc::from("__class__")),
                    PyValue::Str(Arc::from("NullHandler")),
                );
            }
            Ok(handler)
        }),
        // shutdown - shutdown the logging system
        PyBuiltinFunction::new("shutdown", |_args| Ok(PyValue::None)),
    ]
}

/// Create a logger object
fn create_logger(name: &str) -> PyValue {
    let logger_dict = PyDict::new();
    logger_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Logger")));
    logger_dict.setitem(PyKey::Str(Arc::from("name")), PyValue::Str(Arc::from(name)));
    logger_dict.setitem(PyKey::Str(Arc::from("level")), PyValue::Int(0));
    logger_dict.setitem(PyKey::Str(Arc::from("handlers")), PyValue::List(Arc::new(PyList::new())));
    logger_dict.setitem(PyKey::Str(Arc::from("disabled")), PyValue::Bool(false));
    logger_dict.setitem(PyKey::Str(Arc::from("propagate")), PyValue::Bool(true));
    PyValue::Dict(Arc::new(logger_dict))
}

/// Create a handler object
fn create_handler() -> PyValue {
    let handler_dict = PyDict::new();
    handler_dict.setitem(PyKey::Str(Arc::from("__class__")), PyValue::Str(Arc::from("Handler")));
    handler_dict.setitem(PyKey::Str(Arc::from("level")), PyValue::Int(0));
    handler_dict.setitem(PyKey::Str(Arc::from("formatter")), PyValue::None);
    handler_dict.setitem(PyKey::Str(Arc::from("filters")), PyValue::List(Arc::new(PyList::new())));
    PyValue::Dict(Arc::new(handler_dict))
}

/// Log a message at the specified level
fn log_message(level: i64, args: &[PyValue]) -> RuntimeResult<PyValue> {
    let msg = match args.first() {
        Some(PyValue::Str(s)) => s.to_string(),
        Some(v) => format!("{:?}", v),
        None => "".to_string(),
    };

    let level_name = match level {
        50 => "CRITICAL",
        40 => "ERROR",
        30 => "WARNING",
        20 => "INFO",
        10 => "DEBUG",
        _ => "LOG",
    };

    // Print to stderr for now
    eprintln!("{}: {}", level_name, msg);
    Ok(PyValue::None)
}

/// Create logging object method builtins
pub fn logging_object_builtins() -> Vec<PyBuiltinFunction> {
    vec![
        // Logger.debug
        PyBuiltinFunction::new("Logger_debug", |args| {
            if args.len() < 2 {
                return Ok(PyValue::None);
            }
            log_message(10, &args[1..])
        }),
        // Logger.info
        PyBuiltinFunction::new("Logger_info", |args| {
            if args.len() < 2 {
                return Ok(PyValue::None);
            }
            log_message(20, &args[1..])
        }),
        // Logger.warning
        PyBuiltinFunction::new("Logger_warning", |args| {
            if args.len() < 2 {
                return Ok(PyValue::None);
            }
            log_message(30, &args[1..])
        }),
        // Logger.error
        PyBuiltinFunction::new("Logger_error", |args| {
            if args.len() < 2 {
                return Ok(PyValue::None);
            }
            log_message(40, &args[1..])
        }),
        // Logger.critical
        PyBuiltinFunction::new("Logger_critical", |args| {
            if args.len() < 2 {
                return Ok(PyValue::None);
            }
            log_message(50, &args[1..])
        }),
        // Logger.exception
        PyBuiltinFunction::new("Logger_exception", |args| {
            if args.len() < 2 {
                return Ok(PyValue::None);
            }
            log_message(40, &args[1..])
        }),
        // Logger.log
        PyBuiltinFunction::new("Logger_log", |args| {
            if args.len() < 3 {
                return Ok(PyValue::None);
            }
            let level = match &args[1] {
                PyValue::Int(n) => *n,
                _ => return Err(RuntimeError::type_error("int", args[1].type_name())),
            };
            log_message(level, &args[2..])
        }),
        // Logger.setLevel
        PyBuiltinFunction::new("Logger_setLevel", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                let level = args.get(1).cloned().unwrap_or(PyValue::Int(0));
                d.setitem(PyKey::Str(Arc::from("level")), level);
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Logger",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Logger.getEffectiveLevel
        PyBuiltinFunction::new("Logger_getEffectiveLevel", |args| match args.first() {
            Some(PyValue::Dict(d)) => Ok(d.get(&PyKey::Str(Arc::from("level")), PyValue::Int(0))),
            _ => Err(RuntimeError::type_error(
                "Logger",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Logger.isEnabledFor
        PyBuiltinFunction::new("Logger_isEnabledFor", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                let logger_level = match d.get(&PyKey::Str(Arc::from("level")), PyValue::Int(0)) {
                    PyValue::Int(n) => n,
                    _ => 0,
                };
                let check_level = match args.get(1) {
                    Some(PyValue::Int(n)) => *n,
                    _ => 0,
                };
                Ok(PyValue::Bool(check_level >= logger_level))
            }
            _ => Err(RuntimeError::type_error(
                "Logger",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Logger.addHandler
        PyBuiltinFunction::new("Logger_addHandler", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                if let Some(handler) = args.get(1) {
                    if let PyValue::List(handlers) = d.get(
                        &PyKey::Str(Arc::from("handlers")),
                        PyValue::List(Arc::new(PyList::new())),
                    ) {
                        let mut h = handlers.to_vec();
                        h.push(handler.clone());
                        d.setitem(
                            PyKey::Str(Arc::from("handlers")),
                            PyValue::List(Arc::new(PyList::from_values(h))),
                        );
                    }
                }
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Logger",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Logger.removeHandler
        PyBuiltinFunction::new("Logger_removeHandler", |args| match args.first() {
            Some(PyValue::Dict(_)) => Ok(PyValue::None),
            _ => Err(RuntimeError::type_error(
                "Logger",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Handler.setLevel
        PyBuiltinFunction::new("Handler_setLevel", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                let level = args.get(1).cloned().unwrap_or(PyValue::Int(0));
                d.setitem(PyKey::Str(Arc::from("level")), level);
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Handler",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Handler.setFormatter
        PyBuiltinFunction::new("Handler_setFormatter", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                let formatter = args.get(1).cloned().unwrap_or(PyValue::None);
                d.setitem(PyKey::Str(Arc::from("formatter")), formatter);
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Handler",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Handler.addFilter
        PyBuiltinFunction::new("Handler_addFilter", |args| match args.first() {
            Some(PyValue::Dict(d)) => {
                if let Some(filter) = args.get(1) {
                    if let PyValue::List(filters) = d.get(
                        &PyKey::Str(Arc::from("filters")),
                        PyValue::List(Arc::new(PyList::new())),
                    ) {
                        let mut f = filters.to_vec();
                        f.push(filter.clone());
                        d.setitem(
                            PyKey::Str(Arc::from("filters")),
                            PyValue::List(Arc::new(PyList::from_values(f))),
                        );
                    }
                }
                Ok(PyValue::None)
            }
            _ => Err(RuntimeError::type_error(
                "Handler",
                args.first().map(|v| v.type_name()).unwrap_or("nothing"),
            )),
        }),
        // Formatter.format
        PyBuiltinFunction::new("Formatter_format", |args| match args.get(1) {
            Some(PyValue::Dict(record)) => {
                let msg = record.get(&PyKey::Str(Arc::from("msg")), PyValue::Str(Arc::from("")));
                Ok(msg)
            }
            _ => Ok(PyValue::Str(Arc::from(""))),
        }),
    ]
}
