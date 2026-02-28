//! Fast FFI for low-overhead native calls

use crate::teleport::TeleportedArray;
use dashmap::DashMap;

/// Fast FFI for low-overhead C function calls
///
/// Targets <10ns call overhead by:
/// - Caching function pointers
/// - Zero-copy argument passing
/// - Avoiding GIL acquisition for pure computation
pub struct FastFfi {
    /// Cached function pointers by name
    cache: DashMap<String, FfiFunction>,
}

/// A cached FFI function
pub struct FfiFunction {
    /// Function pointer
    ptr: *const (),
    /// Number of arguments
    arg_count: usize,
    /// Whether this function is GIL-free
    gil_free: bool,
}

// Safety: FfiFunction is Send + Sync because function pointers
// point to immutable code
unsafe impl Send for FfiFunction {}
unsafe impl Sync for FfiFunction {}

impl FastFfi {
    /// Create a new FastFfi instance
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Register a function for fast calling
    ///
    /// # Safety
    /// The function pointer must be valid and have the correct signature.
    pub unsafe fn register(&self, name: &str, ptr: *const (), arg_count: usize, gil_free: bool) {
        self.cache.insert(
            name.to_string(),
            FfiFunction {
                ptr,
                arg_count,
                gil_free,
            },
        );
    }

    /// Get a registered function
    pub fn get(&self, name: &str) -> Option<FfiFunction> {
        self.cache.get(name).map(|r| FfiFunction {
            ptr: r.ptr,
            arg_count: r.arg_count,
            gil_free: r.gil_free,
        })
    }

    /// Call a function with zero-copy array arguments
    ///
    /// # Safety
    /// - The function must have the correct signature
    /// - The arrays must have compatible dtypes
    pub unsafe fn call_arrays<R>(&self, name: &str, args: &[&TeleportedArray]) -> Option<R> {
        let func = self.get(name)?;

        if args.len() != func.arg_count {
            return None;
        }

        // Collect data pointers
        let ptrs: Vec<*const u8> = args.iter().map(|a| a.data_ptr()).collect();

        // Call the function
        // This is a simplified implementation - real impl would handle
        // different signatures
        match func.arg_count {
            0 => {
                let f: extern "C" fn() -> R = std::mem::transmute(func.ptr);
                Some(f())
            }
            1 => {
                let f: extern "C" fn(*const u8) -> R = std::mem::transmute(func.ptr);
                Some(f(ptrs[0]))
            }
            2 => {
                let f: extern "C" fn(*const u8, *const u8) -> R = std::mem::transmute(func.ptr);
                Some(f(ptrs[0], ptrs[1]))
            }
            _ => None,
        }
    }

    /// Call a function with scalar arguments
    ///
    /// # Safety
    /// The function must have the correct signature.
    pub unsafe fn call_scalars<R>(&self, name: &str, args: &[u64]) -> Option<R> {
        let func = self.get(name)?;

        if args.len() != func.arg_count {
            return None;
        }

        match func.arg_count {
            0 => {
                let f: extern "C" fn() -> R = std::mem::transmute(func.ptr);
                Some(f())
            }
            1 => {
                let f: extern "C" fn(u64) -> R = std::mem::transmute(func.ptr);
                Some(f(args[0]))
            }
            2 => {
                let f: extern "C" fn(u64, u64) -> R = std::mem::transmute(func.ptr);
                Some(f(args[0], args[1]))
            }
            3 => {
                let f: extern "C" fn(u64, u64, u64) -> R = std::mem::transmute(func.ptr);
                Some(f(args[0], args[1], args[2]))
            }
            _ => None,
        }
    }

    /// Check if a function is registered
    pub fn has(&self, name: &str) -> bool {
        self.cache.contains_key(name)
    }

    /// Remove a registered function
    pub fn remove(&self, name: &str) -> bool {
        self.cache.remove(name).is_some()
    }

    /// Clear all registered functions
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get the number of registered functions
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for FastFfi {
    fn default() -> Self {
        Self::new()
    }
}

/// GIL-free execution context
///
/// Allows executing pure computation without holding the GIL.
pub struct GilFreeContext {
    /// Whether we're currently in GIL-free mode
    active: bool,
}

impl GilFreeContext {
    /// Create a new GIL-free context
    pub fn new() -> Self {
        Self { active: false }
    }

    /// Enter GIL-free mode
    ///
    /// In a real implementation, this would release the GIL.
    pub fn enter(&mut self) {
        self.active = true;
    }

    /// Exit GIL-free mode
    ///
    /// In a real implementation, this would reacquire the GIL.
    pub fn exit(&mut self) {
        self.active = false;
    }

    /// Check if in GIL-free mode
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Execute a closure in GIL-free mode
    pub fn execute<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.enter();
        let result = f();
        self.exit();
        result
    }
}

impl Default for GilFreeContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn test_func_0() -> i32 {
        42
    }

    extern "C" fn test_func_1(a: u64) -> u64 {
        a * 2
    }

    extern "C" fn test_func_2(a: u64, b: u64) -> u64 {
        a + b
    }

    #[test]
    fn test_fast_ffi_register() {
        let ffi = FastFfi::new();

        unsafe {
            ffi.register("test0", test_func_0 as *const (), 0, true);
            ffi.register("test1", test_func_1 as *const (), 1, true);
        }

        assert!(ffi.has("test0"));
        assert!(ffi.has("test1"));
        assert!(!ffi.has("test2"));
    }

    #[test]
    fn test_fast_ffi_call() {
        let ffi = FastFfi::new();

        unsafe {
            ffi.register("test0", test_func_0 as *const (), 0, true);
            ffi.register("test1", test_func_1 as *const (), 1, true);
            ffi.register("test2", test_func_2 as *const (), 2, true);

            let result0: Option<i32> = ffi.call_scalars("test0", &[]);
            assert_eq!(result0, Some(42));

            let result1: Option<u64> = ffi.call_scalars("test1", &[21]);
            assert_eq!(result1, Some(42));

            let result2: Option<u64> = ffi.call_scalars("test2", &[20, 22]);
            assert_eq!(result2, Some(42));
        }
    }

    #[test]
    fn test_gil_free_context() {
        let mut ctx = GilFreeContext::new();

        assert!(!ctx.is_active());

        ctx.enter();
        assert!(ctx.is_active());
        ctx.exit();

        assert!(!ctx.is_active());

        let result = ctx.execute(|| 42);

        assert_eq!(result, 42);
        assert!(!ctx.is_active());
    }
}
