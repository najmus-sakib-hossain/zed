// Client-side runtime integration for dx-client

/// Initialize runtime ecosystem features
#[cfg(feature = "dev")]
pub fn init() {
    // Install panic hook for better error messages
    #[cfg(feature = "dx-error")]
    dx_error::install_panic_hook();

    // Setup debug hooks
    #[cfg(feature = "dx-debug")]
    init_debug_hooks();
}

#[cfg(not(feature = "dev"))]
pub fn init() {
    // Production mode - no extra features
}

/// Initialize debug hooks (dev mode only)
#[cfg(all(feature = "dev", feature = "dx-debug"))]
fn init_debug_hooks() {
    // Expose debug API to window.__DX__
    // This would use wasm-bindgen in production
}

#[cfg(not(all(feature = "dev", feature = "dx-debug")))]
fn init_debug_hooks() {
    // No-op in production
}

/// Form validation runtime (compiled from schemas)
pub mod form {
    /// Validate field (called from generated code)
    pub fn validate_field(value: &str, validators: &[u8]) -> u16 {
        // Validators is a binary array of validator opcodes
        // Returns error bitmask
        let mut errors = 0u16;

        for &validator_opcode in validators {
            match validator_opcode {
                0x01 => {
                    // required
                    if value.is_empty() {
                        errors |= 1 << 0;
                    }
                }
                0x02 => {
                    // email
                    if !value.contains('@') {
                        errors |= 1 << 1;
                    }
                }
                _ => {}
            }
        }

        errors
    }
}

/// Query cache runtime (minimal implementation)
pub mod query {
    use std::collections::HashMap;

    static mut QUERY_CACHE: Option<HashMap<u32, Vec<u8>>> = None;

    /// Initialize query cache
    pub fn init_cache() {
        unsafe {
            QUERY_CACHE = Some(HashMap::new());
        }
    }

    /// Get cached query result
    pub fn get_cached(query_hash: u32) -> Option<&'static [u8]> {
        unsafe { QUERY_CACHE.as_ref()?.get(&query_hash).map(|v| v.as_slice()) }
    }

    /// Cache query result
    pub fn cache_result(query_hash: u32, data: Vec<u8>) {
        unsafe {
            if let Some(cache) = QUERY_CACHE.as_mut() {
                cache.insert(query_hash, data);
            }
        }
    }
}

/// State management runtime (minimal binary state)
pub mod state {
    /// Global state pointer (managed by host)
    static mut STATE_PTR: *mut u8 = std::ptr::null_mut();
    static mut STATE_SIZE: usize = 0;

    /// Initialize state with size
    pub fn init_state(size: usize) {
        unsafe {
            STATE_SIZE = size;
            // State is allocated by host JavaScript
        }
    }

    /// Set state pointer (called from host)
    pub fn set_state_ptr(ptr: *mut u8) {
        unsafe {
            STATE_PTR = ptr;
        }
    }

    /// Read u32 from state at offset
    pub fn read_u32(offset: usize) -> u32 {
        unsafe {
            if STATE_PTR.is_null() || offset + 4 > STATE_SIZE {
                return 0;
            }
            let ptr = STATE_PTR.add(offset) as *const u32;
            *ptr
        }
    }

    /// Write u32 to state at offset
    pub fn write_u32(offset: usize, value: u32) {
        unsafe {
            if STATE_PTR.is_null() || offset + 4 > STATE_SIZE {
                return;
            }
            let ptr = STATE_PTR.add(offset) as *mut u32;
            *ptr = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_validation() {
        let validators = vec![0x01]; // required
        let errors = form::validate_field("", &validators);
        assert_ne!(errors, 0);

        let errors2 = form::validate_field("value", &validators);
        assert_eq!(errors2, 0);
    }

    #[test]
    fn test_query_cache() {
        query::init_cache();
        query::cache_result(123, vec![1, 2, 3]);

        let cached = query::get_cached(123);
        assert_eq!(cached, Some(&[1, 2, 3][..]));
    }
}
