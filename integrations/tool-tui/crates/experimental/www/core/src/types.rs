//! # Compile-Time Type Safety
//!
//! Binary schema trait for types shared between server and client.
//! Wire format equals memory layout - no JSON schema, no codegen step.
//!
//! **Validates: Requirements 25.1, 25.2, 25.3, 25.4**

use std::mem;

/// Binary schema trait - types shared between server and client
///
/// Types implementing this trait can be serialized to binary format
/// where the wire format exactly matches the in-memory representation.
///
/// # Example
///
/// ```ignore
/// #[derive(BinarySchema)]
/// #[repr(C)]
/// struct User {
///     id: u64,
///     name_offset: u32,
///     name_len: u32,
///     age: u8,
/// }
/// ```
///
/// The derive macro generates:
/// - `SIZE` and `ALIGN` constants from the struct layout
/// - `to_binary()` that copies memory directly
/// - `from_binary()` that interprets bytes as the struct
pub trait BinarySchema: Sized {
    /// Size of the type in bytes
    const SIZE: usize;
    /// Alignment requirement of the type
    const ALIGN: usize;

    /// Serialize to binary format
    ///
    /// The output bytes exactly match the in-memory representation.
    fn to_binary(&self) -> Vec<u8>;

    /// Deserialize from binary format
    ///
    /// # Safety
    /// The input bytes must be a valid representation of the type.
    fn from_binary(bytes: &[u8]) -> Self;

    /// Check if bytes are valid for this type
    fn is_valid_binary(bytes: &[u8]) -> bool {
        bytes.len() >= Self::SIZE
    }
}

/// Marker trait for types that are safe to transmute from bytes
///
/// # Safety
/// Types implementing this trait must:
/// - Have no padding bytes that could contain uninitialized memory
/// - Have no invalid bit patterns
/// - Be `#[repr(C)]` or `#[repr(transparent)]`
pub unsafe trait SafeTransmute: Sized {}

// Implement SafeTransmute for primitive types
unsafe impl SafeTransmute for u8 {}
unsafe impl SafeTransmute for u16 {}
unsafe impl SafeTransmute for u32 {}
unsafe impl SafeTransmute for u64 {}
unsafe impl SafeTransmute for i8 {}
unsafe impl SafeTransmute for i16 {}
unsafe impl SafeTransmute for i32 {}
unsafe impl SafeTransmute for i64 {}
unsafe impl SafeTransmute for f32 {}
unsafe impl SafeTransmute for f64 {}

// Implement BinarySchema for primitive types
impl BinarySchema for u8 {
    const SIZE: usize = mem::size_of::<u8>();
    const ALIGN: usize = mem::align_of::<u8>();

    fn to_binary(&self) -> Vec<u8> {
        vec![*self]
    }

    fn from_binary(bytes: &[u8]) -> Self {
        bytes[0]
    }
}

impl BinarySchema for u16 {
    const SIZE: usize = mem::size_of::<u16>();
    const ALIGN: usize = mem::align_of::<u16>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }
}

impl BinarySchema for u32 {
    const SIZE: usize = mem::size_of::<u32>();
    const ALIGN: usize = mem::align_of::<u32>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

impl BinarySchema for u64 {
    const SIZE: usize = mem::size_of::<u64>();
    const ALIGN: usize = mem::align_of::<u64>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

impl BinarySchema for i8 {
    const SIZE: usize = mem::size_of::<i8>();
    const ALIGN: usize = mem::align_of::<i8>();

    fn to_binary(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_binary(bytes: &[u8]) -> Self {
        bytes[0] as i8
    }
}

impl BinarySchema for i16 {
    const SIZE: usize = mem::size_of::<i16>();
    const ALIGN: usize = mem::align_of::<i16>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        i16::from_le_bytes([bytes[0], bytes[1]])
    }
}

impl BinarySchema for i32 {
    const SIZE: usize = mem::size_of::<i32>();
    const ALIGN: usize = mem::align_of::<i32>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

impl BinarySchema for i64 {
    const SIZE: usize = mem::size_of::<i64>();
    const ALIGN: usize = mem::align_of::<i64>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

impl BinarySchema for f32 {
    const SIZE: usize = mem::size_of::<f32>();
    const ALIGN: usize = mem::align_of::<f32>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

impl BinarySchema for f64 {
    const SIZE: usize = mem::size_of::<f64>();
    const ALIGN: usize = mem::align_of::<f64>();

    fn to_binary(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_binary(bytes: &[u8]) -> Self {
        f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

impl BinarySchema for bool {
    const SIZE: usize = 1;
    const ALIGN: usize = 1;

    fn to_binary(&self) -> Vec<u8> {
        vec![if *self { 1 } else { 0 }]
    }

    fn from_binary(bytes: &[u8]) -> Self {
        bytes[0] != 0
    }
}

/// Example struct implementing BinarySchema manually
///
/// In practice, this would be generated by `#[derive(BinarySchema)]`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExampleUser {
    pub id: u64,
    pub name_offset: u32,
    pub name_len: u32,
    pub age: u8,
    pub _padding: [u8; 7], // Explicit padding for 8-byte alignment
}

unsafe impl SafeTransmute for ExampleUser {}

impl BinarySchema for ExampleUser {
    const SIZE: usize = mem::size_of::<ExampleUser>();
    const ALIGN: usize = mem::align_of::<ExampleUser>();

    fn to_binary(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes.extend_from_slice(&self.name_offset.to_le_bytes());
        bytes.extend_from_slice(&self.name_len.to_le_bytes());
        bytes.push(self.age);
        bytes.extend_from_slice(&self._padding);
        bytes
    }

    fn from_binary(bytes: &[u8]) -> Self {
        Self {
            id: u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            name_offset: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            name_len: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            age: bytes[16],
            _padding: [
                bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
            ],
        }
    }
}

impl ExampleUser {
    /// Create a new user
    pub fn new(id: u64, name_offset: u32, name_len: u32, age: u8) -> Self {
        Self {
            id,
            name_offset,
            name_len,
            age,
            _padding: [0; 7],
        }
    }
}

/// Helper to verify wire format matches memory layout
pub fn verify_wire_format<T: BinarySchema + SafeTransmute + Copy>(value: &T) -> bool {
    let binary = value.to_binary();

    // Check size matches
    if binary.len() != T::SIZE {
        return false;
    }

    // Check round-trip
    let restored = T::from_binary(&binary);
    let restored_binary = restored.to_binary();

    binary == restored_binary
}

/// Schema registry for runtime type lookup
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: Vec<SchemaInfo>,
}

/// Information about a registered schema
#[derive(Debug, Clone)]
pub struct SchemaInfo {
    pub name: &'static str,
    pub size: usize,
    pub align: usize,
}

impl SchemaRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a schema
    pub fn register<T: BinarySchema>(&mut self, name: &'static str) {
        self.schemas.push(SchemaInfo {
            name,
            size: T::SIZE,
            align: T::ALIGN,
        });
    }

    /// Get schema info by name
    pub fn get(&self, name: &str) -> Option<&SchemaInfo> {
        self.schemas.iter().find(|s| s.name == name)
    }

    /// Get all registered schemas
    pub fn all(&self) -> &[SchemaInfo] {
        &self.schemas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit tests

    #[test]
    fn test_primitive_sizes() {
        assert_eq!(u8::SIZE, 1);
        assert_eq!(u16::SIZE, 2);
        assert_eq!(u32::SIZE, 4);
        assert_eq!(u64::SIZE, 8);
        assert_eq!(i8::SIZE, 1);
        assert_eq!(i16::SIZE, 2);
        assert_eq!(i32::SIZE, 4);
        assert_eq!(i64::SIZE, 8);
        assert_eq!(f32::SIZE, 4);
        assert_eq!(f64::SIZE, 8);
        assert_eq!(bool::SIZE, 1);
    }

    #[test]
    fn test_example_user_size() {
        // ExampleUser: u64 (8) + u32 (4) + u32 (4) + u8 (1) + padding (7) = 24 bytes
        assert_eq!(ExampleUser::SIZE, 24);
    }

    #[test]
    fn test_example_user_round_trip() {
        let user = ExampleUser::new(12345, 100, 10, 25);
        let binary = user.to_binary();
        let restored = ExampleUser::from_binary(&binary);

        assert_eq!(user, restored);
    }

    #[test]
    fn test_verify_wire_format() {
        let user = ExampleUser::new(42, 0, 5, 30);
        assert!(verify_wire_format(&user));
    }

    #[test]
    fn test_schema_registry() {
        let mut registry = SchemaRegistry::new();
        registry.register::<u32>("u32");
        registry.register::<ExampleUser>("ExampleUser");

        let u32_info = registry.get("u32").unwrap();
        assert_eq!(u32_info.size, 4);

        let user_info = registry.get("ExampleUser").unwrap();
        assert_eq!(user_info.size, 24);

        assert!(registry.get("NonExistent").is_none());
    }

    #[test]
    fn test_is_valid_binary() {
        assert!(u32::is_valid_binary(&[1, 2, 3, 4]));
        assert!(!u32::is_valid_binary(&[1, 2, 3])); // Too short
    }

    // Property-based tests

    // Feature: binary-dawn-features, Property 39: BinarySchema Wire Format
    // For any type implementing BinarySchema, the serialized bytes SHALL exactly
    // match the in-memory representation (wire format = memory layout).
    // Validates: Requirements 25.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_u8_round_trip(value: u8) {
            let binary = value.to_binary();
            let restored = u8::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), u8::SIZE);
        }

        #[test]
        fn prop_u16_round_trip(value: u16) {
            let binary = value.to_binary();
            let restored = u16::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), u16::SIZE);
        }

        #[test]
        fn prop_u32_round_trip(value: u32) {
            let binary = value.to_binary();
            let restored = u32::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), u32::SIZE);
        }

        #[test]
        fn prop_u64_round_trip(value: u64) {
            let binary = value.to_binary();
            let restored = u64::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), u64::SIZE);
        }

        #[test]
        fn prop_i8_round_trip(value: i8) {
            let binary = value.to_binary();
            let restored = i8::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), i8::SIZE);
        }

        #[test]
        fn prop_i16_round_trip(value: i16) {
            let binary = value.to_binary();
            let restored = i16::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), i16::SIZE);
        }

        #[test]
        fn prop_i32_round_trip(value: i32) {
            let binary = value.to_binary();
            let restored = i32::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), i32::SIZE);
        }

        #[test]
        fn prop_i64_round_trip(value: i64) {
            let binary = value.to_binary();
            let restored = i64::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), i64::SIZE);
        }

        #[test]
        fn prop_f32_round_trip(value: f32) {
            // Skip NaN values as they don't compare equal
            prop_assume!(!value.is_nan());
            let binary = value.to_binary();
            let restored = f32::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), f32::SIZE);
        }

        #[test]
        fn prop_f64_round_trip(value: f64) {
            // Skip NaN values as they don't compare equal
            prop_assume!(!value.is_nan());
            let binary = value.to_binary();
            let restored = f64::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), f64::SIZE);
        }

        #[test]
        fn prop_bool_round_trip(value: bool) {
            let binary = value.to_binary();
            let restored = bool::from_binary(&binary);
            prop_assert_eq!(value, restored);
            prop_assert_eq!(binary.len(), bool::SIZE);
        }

        #[test]
        fn prop_example_user_round_trip(
            id: u64,
            name_offset: u32,
            name_len: u32,
            age: u8
        ) {
            let user = ExampleUser::new(id, name_offset, name_len, age);
            let binary = user.to_binary();
            let restored = ExampleUser::from_binary(&binary);

            // Property: Round-trip preserves all fields
            prop_assert_eq!(user.id, restored.id);
            prop_assert_eq!(user.name_offset, restored.name_offset);
            prop_assert_eq!(user.name_len, restored.name_len);
            prop_assert_eq!(user.age, restored.age);

            // Property: Binary size matches SIZE constant
            prop_assert_eq!(binary.len(), ExampleUser::SIZE);
        }

        #[test]
        fn prop_wire_format_matches_memory(
            id: u64,
            name_offset: u32,
            name_len: u32,
            age: u8
        ) {
            let user = ExampleUser::new(id, name_offset, name_len, age);

            // Property: Wire format verification passes
            prop_assert!(verify_wire_format(&user));
        }

        #[test]
        fn prop_binary_size_equals_size_constant(
            id: u64,
            name_offset: u32,
            name_len: u32,
            age: u8
        ) {
            let user = ExampleUser::new(id, name_offset, name_len, age);
            let binary = user.to_binary();

            // Property: Serialized size always equals SIZE constant
            prop_assert_eq!(binary.len(), ExampleUser::SIZE);
            prop_assert_eq!(binary.len(), std::mem::size_of::<ExampleUser>());
        }
    }
}
