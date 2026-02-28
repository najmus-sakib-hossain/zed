//! Dynamic library loading and symbol resolution.
//!
//! Provides cross-platform dynamic library loading with support for:
//! - Windows DLLs (.dll)
//! - macOS dynamic libraries (.dylib)
//! - Linux/Unix shared objects (.so)

use crate::error::{FfiError, FfiResult};
use libloading::{Library, Symbol};
use std::path::Path;

/// Load a dynamic library from the given path.
///
/// # Platform-specific behavior
/// - Windows: Loads .dll files
/// - macOS: Loads .dylib files
/// - Linux: Loads .so files
///
/// # Example
/// ```ignore
/// let lib = dlopen("libfoo.so")?;
/// ```
pub fn dlopen(path: &str) -> FfiResult<DynamicLibrary> {
    DynamicLibrary::open(path)
}

/// Dynamic library handle for FFI operations.
pub struct DynamicLibrary {
    lib: Library,
    path: String,
}

impl DynamicLibrary {
    /// Open a dynamic library.
    pub fn open(path: impl AsRef<Path>) -> FfiResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let lib = unsafe { Library::new(path.as_ref())? };
        Ok(Self {
            lib,
            path: path_str,
        })
    }

    /// Get the library path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get a function symbol from the library.
    ///
    /// # Safety
    /// The caller must ensure:
    /// - The symbol type `T` matches the actual function signature
    /// - The function is called with correct arguments
    ///
    /// # Example
    /// ```ignore
    /// type AddFn = unsafe extern "C" fn(i32, i32) -> i32;
    /// let add: Symbol<AddFn> = unsafe { lib.get("add")? };
    /// let result = unsafe { add(1, 2) };
    /// ```
    pub unsafe fn get<T>(&self, name: &str) -> FfiResult<Symbol<'_, T>> {
        unsafe {
            self.lib
                .get(name.as_bytes())
                .map_err(|_| FfiError::SymbolNotFound(name.to_string()))
        }
    }

    /// Get a function symbol, returning None if not found.
    ///
    /// # Safety
    /// Same requirements as `get`.
    pub unsafe fn try_get<T>(&self, name: &str) -> Option<Symbol<'_, T>> {
        unsafe { self.lib.get(name.as_bytes()).ok() }
    }

    /// Check if a symbol exists in the library.
    pub fn has_symbol(&self, name: &str) -> bool {
        unsafe { self.lib.get::<*const ()>(name.as_bytes()).is_ok() }
    }

    /// Close the library explicitly.
    ///
    /// Note: The library is also closed when dropped.
    pub fn close(self) {
        drop(self.lib);
    }
}

/// Suffix for dynamic libraries on the current platform.
pub fn library_suffix() -> &'static str {
    if cfg!(target_os = "windows") {
        ".dll"
    } else if cfg!(target_os = "macos") {
        ".dylib"
    } else {
        ".so"
    }
}

/// Prefix for dynamic libraries on the current platform.
pub fn library_prefix() -> &'static str {
    if cfg!(target_os = "windows") {
        ""
    } else {
        "lib"
    }
}

/// Build a platform-specific library name.
///
/// # Example
/// ```
/// use dx_compat_ffi::library_name;
/// // On Linux: "libfoo.so"
/// // On macOS: "libfoo.dylib"
/// // On Windows: "foo.dll"
/// let name = library_name("foo");
/// ```
pub fn library_name(base_name: &str) -> String {
    format!("{}{}{}", library_prefix(), base_name, library_suffix())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_suffix() {
        let suffix = library_suffix();
        assert!(suffix == ".dll" || suffix == ".dylib" || suffix == ".so");
    }

    #[test]
    fn test_library_name() {
        let name = library_name("test");
        assert!(name.contains("test"));
    }

    #[test]
    fn test_dlopen_nonexistent() {
        let result = dlopen("nonexistent_library_12345.so");
        assert!(result.is_err());
    }
}
