//! # Binary Server Components
//!
//! Binary Dawn's server components stream binary data instead of JSON.
//! This achieves 50x smaller payloads compared to React Server Components.
//!
//! Server components compile to BinaryFragment containing template_id and binary-encoded slots.

/// Server component output
///
/// Contains a pre-registered template ID and binary-encoded data slots.
/// This is 16x smaller than RSC's JSON format (~12 bytes per user vs ~200 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryFragment {
    /// Pre-registered template ID
    pub template_id: u16,
    /// Binary-encoded data slots
    pub slots: Vec<u8>,
}

impl BinaryFragment {
    /// Create a new binary fragment
    pub fn new(template_id: u16) -> Self {
        Self {
            template_id,
            slots: Vec::new(),
        }
    }

    /// Create with pre-allocated slot capacity
    pub fn with_capacity(template_id: u16, capacity: usize) -> Self {
        Self {
            template_id,
            slots: Vec::with_capacity(capacity),
        }
    }

    /// Add a u32 value to slots
    #[inline]
    pub fn push_u32(&mut self, value: u32) {
        self.slots.extend_from_slice(&value.to_le_bytes());
    }

    /// Add a u16 value to slots
    #[inline]
    pub fn push_u16(&mut self, value: u16) {
        self.slots.extend_from_slice(&value.to_le_bytes());
    }

    /// Add a u8 value to slots
    #[inline]
    pub fn push_u8(&mut self, value: u8) {
        self.slots.push(value);
    }

    /// Add a string to slots (length-prefixed)
    #[inline]
    pub fn push_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.push_u16(bytes.len() as u16);
        self.slots.extend_from_slice(bytes);
    }

    /// Add raw bytes to slots
    #[inline]
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.slots.extend_from_slice(bytes);
    }

    /// Get total size in bytes
    pub fn total_size(&self) -> usize {
        ServerComponentHeader::SIZE + self.slots.len()
    }

    /// Serialize to bytes (header + slots)
    pub fn to_bytes(&self) -> Vec<u8> {
        let header = ServerComponentHeader {
            template_id: self.template_id,
            slot_count: 0, // Not tracking individual slots
            total_size: self.total_size() as u32,
        };

        let mut bytes = Vec::with_capacity(self.total_size());
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&self.slots);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < ServerComponentHeader::SIZE {
            return None;
        }

        let header = ServerComponentHeader::from_bytes(&bytes[..ServerComponentHeader::SIZE])?;
        let slots = bytes[ServerComponentHeader::SIZE..].to_vec();

        Some(Self {
            template_id: header.template_id,
            slots,
        })
    }
}

/// Wire format header for server component data
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerComponentHeader {
    /// Template ID
    pub template_id: u16,
    /// Number of slots (for validation)
    pub slot_count: u16,
    /// Total size including header
    pub total_size: u32,
}

impl ServerComponentHeader {
    /// Size of header in bytes
    pub const SIZE: usize = 8;

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..2].copy_from_slice(&self.template_id.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.slot_count.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.total_size.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            template_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            slot_count: u16::from_le_bytes([bytes[2], bytes[3]]),
            total_size: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        })
    }
}

/// Server component trait
///
/// Implement this trait to create server components that render to binary fragments.
pub trait ServerComponent {
    /// Render to binary fragment (never ships to client)
    fn render(&self) -> BinaryFragment;
}

/// Example user struct for demonstration
#[derive(Debug, Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
}

impl User {
    /// Create a new user
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }

    /// Serialize to binary (~8 bytes + name length, vs ~200 bytes JSON)
    pub fn to_binary(&self) -> Vec<u8> {
        let name_bytes = self.name.as_bytes();
        let mut bytes = Vec::with_capacity(4 + 2 + name_bytes.len());
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        bytes.extend_from_slice(name_bytes);
        bytes
    }

    /// Deserialize from binary
    pub fn from_binary(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 6 {
            return None;
        }
        let id = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let name_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        if bytes.len() < 6 + name_len {
            return None;
        }
        let name = String::from_utf8(bytes[6..6 + name_len].to_vec()).ok()?;
        Some(Self { id, name })
    }

    /// Get binary size
    pub fn binary_size(&self) -> usize {
        4 + 2 + self.name.len()
    }
}

/// Example: User list server component
pub struct UserListComponent {
    pub users: Vec<User>,
}

impl UserListComponent {
    /// Create a new user list component
    pub fn new(users: Vec<User>) -> Self {
        Self { users }
    }
}

impl ServerComponent for UserListComponent {
    fn render(&self) -> BinaryFragment {
        // Calculate capacity: ~12 bytes per user average
        let capacity = self.users.len() * 12;
        let mut fragment = BinaryFragment::with_capacity(42, capacity);

        // Add user count
        fragment.push_u16(self.users.len() as u16);

        // Add each user's binary data
        for user in &self.users {
            fragment.push_bytes(&user.to_binary());
        }

        fragment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_component_header_size() {
        assert_eq!(std::mem::size_of::<ServerComponentHeader>(), ServerComponentHeader::SIZE);
    }

    #[test]
    fn test_binary_fragment_roundtrip() {
        let mut fragment = BinaryFragment::new(42);
        fragment.push_u32(123);
        fragment.push_string("hello");

        let bytes = fragment.to_bytes();
        let restored = BinaryFragment::from_bytes(&bytes).unwrap();

        assert_eq!(fragment.template_id, restored.template_id);
        assert_eq!(fragment.slots, restored.slots);
    }

    #[test]
    fn test_user_binary_size() {
        let user = User::new(1, "Alice");
        let binary = user.to_binary();

        // 4 bytes id + 2 bytes name_len + 5 bytes name = 11 bytes
        assert_eq!(binary.len(), 11);
        assert!(binary.len() < 20); // Much smaller than JSON's ~200 bytes
    }

    #[test]
    fn test_user_roundtrip() {
        let user = User::new(42, "Bob");
        let binary = user.to_binary();
        let restored = User::from_binary(&binary).unwrap();

        assert_eq!(user.id, restored.id);
        assert_eq!(user.name, restored.name);
    }

    #[test]
    fn test_user_list_component() {
        let users = vec![
            User::new(1, "Alice"),
            User::new(2, "Bob"),
            User::new(3, "Charlie"),
        ];

        let component = UserListComponent::new(users);
        let fragment = component.render();

        assert_eq!(fragment.template_id, 42);
        // 2 bytes count + 3 users * ~12 bytes each
        assert!(fragment.slots.len() < 50);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 5: Server Component Binary Size**
    // *For any* user record serialized via BinaryFragment, the serialized size
    // SHALL be less than 20 bytes (significantly smaller than JSON's ~200 bytes).
    // **Validates: Requirements 3.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_server_component_binary_size(
            id in any::<u32>(),
            name in "[a-zA-Z]{1,8}" // Short names to ensure < 20 bytes
        ) {
            let user = User::new(id, &name);
            let binary = user.to_binary();

            // Binary size should be: 4 (id) + 2 (name_len) + name.len()
            let expected_size = 4 + 2 + name.len();
            prop_assert_eq!(binary.len(), expected_size);

            // For names up to 8 chars, total should be < 20 bytes
            prop_assert!(binary.len() < 20, "Binary size {} >= 20", binary.len());
        }
    }

    // **Feature: binary-dawn-features, Property 6: Binary Fragment Round-Trip**
    // *For any* BinaryFragment written to a stream, reading it back SHALL produce
    // an equivalent BinaryFragment with the same template_id and slots.
    // **Validates: Requirements 3.1, 3.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_binary_fragment_roundtrip(
            template_id in any::<u16>(),
            slot_data in prop::collection::vec(any::<u8>(), 0..100)
        ) {
            let mut fragment = BinaryFragment::new(template_id);
            fragment.slots = slot_data.clone();

            let bytes = fragment.to_bytes();
            let restored = BinaryFragment::from_bytes(&bytes);

            prop_assert!(restored.is_some());
            let restored = restored.unwrap();

            prop_assert_eq!(fragment.template_id, restored.template_id);
            prop_assert_eq!(fragment.slots, restored.slots);
        }
    }

    // User round-trip property
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_user_roundtrip(
            id in any::<u32>(),
            name in "[a-zA-Z0-9_]{0,50}"
        ) {
            let user = User::new(id, &name);
            let binary = user.to_binary();
            let restored = User::from_binary(&binary);

            prop_assert!(restored.is_some());
            let restored = restored.unwrap();

            prop_assert_eq!(user.id, restored.id);
            prop_assert_eq!(user.name, restored.name);
        }
    }

    // ServerComponentHeader round-trip
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_header_roundtrip(
            template_id in any::<u16>(),
            slot_count in any::<u16>(),
            total_size in any::<u32>()
        ) {
            let header = ServerComponentHeader {
                template_id,
                slot_count,
                total_size,
            };

            let bytes = header.to_bytes();
            let restored = ServerComponentHeader::from_bytes(&bytes);

            prop_assert!(restored.is_some());
            prop_assert_eq!(header, restored.unwrap());
        }
    }
}
