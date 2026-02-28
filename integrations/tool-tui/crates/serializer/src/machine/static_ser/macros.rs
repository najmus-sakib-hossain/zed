//! Procedural macros for compile-time serialization.
//!
//! This module provides macros that enable serialization during compilation,
//! eliminating runtime overhead for static data.
//!
//! # Macros
//!
//! - `dx_static_serialize!`: Serialize a value at compile time
//! - `include_serialized!`: Load and serialize a file at compile time
//!
//! # Implementation
//!
//! These macros are implemented as procedural macros in the dx-serializer-derive crate.
//! They are re-exported from the main crate when the "derive" feature is enabled.

/// Helper function for const evaluation of serialization.
///
/// This function is used internally by the macros to perform serialization
/// in a const context. It's not intended for direct use.
///
/// # Safety
///
/// This function is safe because it only performs serialization, which
/// doesn't involve unsafe operations in the RKYV implementation.
#[doc(hidden)]
pub const fn const_serialize<T>(_value: &T) -> &'static [u8]
where
    T: rkyv::Archive,
{
    // TODO: Implement const serialization when const trait impls are stable
    // For now, this is a placeholder that will be replaced by the proc macro
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_serialize_placeholder() {
        // This test verifies the placeholder function exists
        // Real tests will be added when the proc macro is implemented
        let result: &[u8] = const_serialize(&42u32);
        assert_eq!(result, &[] as &[u8]);
    }
}
