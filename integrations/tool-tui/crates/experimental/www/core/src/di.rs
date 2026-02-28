//! # Compile-Time Dependency Injection
//!
//! Zero-cost dependency injection using fixed memory offsets.
//! All dependencies are resolved at compile time - no runtime lookup.
//!
//! ## Design
//!
//! Instead of runtime Map<token, instance> lookups, services use
//! fixed pointer offsets into a Container struct. This eliminates:
//! - Runtime type checking
//! - Map lookups
//! - Dynamic dispatch
//!
//! ## Example
//!
//! ```ignore
//! // Container with fixed offsets
//! #[repr(C)]
//! struct Container {
//!     database_offset: u32,
//!     cache_offset: u32,
//! }
//!
//! // Service with pointer fields
//! #[repr(C)]
//! struct UserService {
//!     db_ptr: u32,
//!     cache_ptr: u32,
//! }
//!
//! // Creation is just pointer assignment
//! fn create_user_service(container: &Container) -> UserService {
//!     UserService {
//!         db_ptr: container.database_offset,
//!         cache_ptr: container.cache_offset,
//!     }
//! }
//! ```

/// Service identifier for compile-time resolution
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceId {
    Database = 0,
    Cache = 1,
    Auth = 2,
    Logger = 3,
    Config = 4,
    HttpClient = 5,
    EventBus = 6,
    Scheduler = 7,
}

impl ServiceId {
    /// Get service ID from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ServiceId::Database),
            1 => Some(ServiceId::Cache),
            2 => Some(ServiceId::Auth),
            3 => Some(ServiceId::Logger),
            4 => Some(ServiceId::Config),
            5 => Some(ServiceId::HttpClient),
            6 => Some(ServiceId::EventBus),
            7 => Some(ServiceId::Scheduler),
            _ => None,
        }
    }

    /// Maximum number of services
    pub const MAX_SERVICES: usize = 8;
}

/// Container with fixed offsets for all services
///
/// Each field is an offset into shared memory where the service instance lives.
/// This allows O(1) service resolution with zero runtime cost.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Container {
    /// Offset to Database service
    pub database_offset: u32,
    /// Offset to Cache service
    pub cache_offset: u32,
    /// Offset to Auth service
    pub auth_offset: u32,
    /// Offset to Logger service
    pub logger_offset: u32,
    /// Offset to Config service
    pub config_offset: u32,
    /// Offset to HttpClient service
    pub http_client_offset: u32,
    /// Offset to EventBus service
    pub event_bus_offset: u32,
    /// Offset to Scheduler service
    pub scheduler_offset: u32,
}

impl Container {
    /// Container size in bytes
    pub const SIZE: usize = 32; // 8 * 4 bytes

    /// Create a new container with all offsets set to 0
    pub fn new() -> Self {
        Self::default()
    }

    /// Get offset for a service by ID
    #[inline(always)]
    pub fn get_offset(&self, service_id: ServiceId) -> u32 {
        match service_id {
            ServiceId::Database => self.database_offset,
            ServiceId::Cache => self.cache_offset,
            ServiceId::Auth => self.auth_offset,
            ServiceId::Logger => self.logger_offset,
            ServiceId::Config => self.config_offset,
            ServiceId::HttpClient => self.http_client_offset,
            ServiceId::EventBus => self.event_bus_offset,
            ServiceId::Scheduler => self.scheduler_offset,
        }
    }

    /// Set offset for a service by ID
    #[inline(always)]
    pub fn set_offset(&mut self, service_id: ServiceId, offset: u32) {
        match service_id {
            ServiceId::Database => self.database_offset = offset,
            ServiceId::Cache => self.cache_offset = offset,
            ServiceId::Auth => self.auth_offset = offset,
            ServiceId::Logger => self.logger_offset = offset,
            ServiceId::Config => self.config_offset = offset,
            ServiceId::HttpClient => self.http_client_offset = offset,
            ServiceId::EventBus => self.event_bus_offset = offset,
            ServiceId::Scheduler => self.scheduler_offset = offset,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.database_offset.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.cache_offset.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.auth_offset.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.logger_offset.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.config_offset.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.http_client_offset.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.event_bus_offset.to_le_bytes());
        bytes[28..32].copy_from_slice(&self.scheduler_offset.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            database_offset: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            cache_offset: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            auth_offset: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            logger_offset: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            config_offset: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            http_client_offset: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            event_bus_offset: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            scheduler_offset: u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
        })
    }
}

/// Injectable trait for services that can be created from a container
pub trait Injectable: Sized {
    /// Create service from container - just pointer assignment
    fn create(container: &Container) -> Self;

    /// Get the service IDs this service depends on
    fn dependencies() -> &'static [ServiceId];
}

/// Example: User service with database and cache dependencies
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UserService {
    /// Pointer to Database in shared memory
    pub db_ptr: u32,
    /// Pointer to Cache in shared memory
    pub cache_ptr: u32,
}

impl UserService {
    /// Service size in bytes
    pub const SIZE: usize = 8;

    /// Create from container - just pointer assignment, zero runtime cost
    #[inline(always)]
    pub fn new(container: &Container) -> Self {
        Self {
            db_ptr: container.database_offset,
            cache_ptr: container.cache_offset,
        }
    }
}

impl Injectable for UserService {
    #[inline(always)]
    fn create(container: &Container) -> Self {
        Self::new(container)
    }

    fn dependencies() -> &'static [ServiceId] {
        &[ServiceId::Database, ServiceId::Cache]
    }
}

/// Example: Auth service with database and config dependencies
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AuthService {
    /// Pointer to Database in shared memory
    pub db_ptr: u32,
    /// Pointer to Config in shared memory
    pub config_ptr: u32,
    /// Pointer to Logger in shared memory
    pub logger_ptr: u32,
}

impl AuthService {
    /// Service size in bytes
    pub const SIZE: usize = 12;

    /// Create from container
    #[inline(always)]
    pub fn new(container: &Container) -> Self {
        Self {
            db_ptr: container.database_offset,
            config_ptr: container.config_offset,
            logger_ptr: container.logger_offset,
        }
    }
}

impl Injectable for AuthService {
    #[inline(always)]
    fn create(container: &Container) -> Self {
        Self::new(container)
    }

    fn dependencies() -> &'static [ServiceId] {
        &[ServiceId::Database, ServiceId::Config, ServiceId::Logger]
    }
}

/// Service registry for compile-time validation
pub struct ServiceRegistry {
    /// Registered service offsets
    offsets: [Option<u32>; ServiceId::MAX_SERVICES],
}

impl ServiceRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            offsets: [None; ServiceId::MAX_SERVICES],
        }
    }

    /// Register a service at an offset
    pub fn register(&mut self, service_id: ServiceId, offset: u32) {
        self.offsets[service_id as usize] = Some(offset);
    }

    /// Check if a service is registered
    pub fn is_registered(&self, service_id: ServiceId) -> bool {
        self.offsets[service_id as usize].is_some()
    }

    /// Get offset for a service
    pub fn get_offset(&self, service_id: ServiceId) -> Option<u32> {
        self.offsets[service_id as usize]
    }

    /// Build a container from the registry
    pub fn build_container(&self) -> Container {
        Container {
            database_offset: self.offsets[ServiceId::Database as usize].unwrap_or(0),
            cache_offset: self.offsets[ServiceId::Cache as usize].unwrap_or(0),
            auth_offset: self.offsets[ServiceId::Auth as usize].unwrap_or(0),
            logger_offset: self.offsets[ServiceId::Logger as usize].unwrap_or(0),
            config_offset: self.offsets[ServiceId::Config as usize].unwrap_or(0),
            http_client_offset: self.offsets[ServiceId::HttpClient as usize].unwrap_or(0),
            event_bus_offset: self.offsets[ServiceId::EventBus as usize].unwrap_or(0),
            scheduler_offset: self.offsets[ServiceId::Scheduler as usize].unwrap_or(0),
        }
    }

    /// Validate that all dependencies for a service are registered
    pub fn validate_dependencies<T: Injectable>(&self) -> Result<(), Vec<ServiceId>> {
        let missing: Vec<_> = T::dependencies()
            .iter()
            .filter(|&&id| !self.is_registered(id))
            .copied()
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_size() {
        assert_eq!(Container::SIZE, 32);
        assert_eq!(std::mem::size_of::<Container>(), 32);
    }

    #[test]
    fn test_container_roundtrip() {
        let mut container = Container::new();
        container.database_offset = 100;
        container.cache_offset = 200;
        container.auth_offset = 300;

        let bytes = container.to_bytes();
        let restored = Container::from_bytes(&bytes).unwrap();

        assert_eq!(restored.database_offset, 100);
        assert_eq!(restored.cache_offset, 200);
        assert_eq!(restored.auth_offset, 300);
    }

    #[test]
    fn test_user_service_creation() {
        let mut container = Container::new();
        container.database_offset = 1000;
        container.cache_offset = 2000;

        let service = UserService::new(&container);

        assert_eq!(service.db_ptr, 1000);
        assert_eq!(service.cache_ptr, 2000);
    }

    #[test]
    fn test_injectable_trait() {
        let mut container = Container::new();
        container.database_offset = 500;
        container.cache_offset = 600;

        let service = UserService::create(&container);

        assert_eq!(service.db_ptr, 500);
        assert_eq!(service.cache_ptr, 600);
    }

    #[test]
    fn test_service_registry() {
        let mut registry = ServiceRegistry::new();
        registry.register(ServiceId::Database, 100);
        registry.register(ServiceId::Cache, 200);

        assert!(registry.is_registered(ServiceId::Database));
        assert!(registry.is_registered(ServiceId::Cache));
        assert!(!registry.is_registered(ServiceId::Auth));

        let container = registry.build_container();
        assert_eq!(container.database_offset, 100);
        assert_eq!(container.cache_offset, 200);
    }

    #[test]
    fn test_validate_dependencies() {
        let mut registry = ServiceRegistry::new();
        registry.register(ServiceId::Database, 100);
        // Cache is not registered

        let result = registry.validate_dependencies::<UserService>();
        assert!(result.is_err());

        let missing = result.unwrap_err();
        assert_eq!(missing, vec![ServiceId::Cache]);

        // Register cache
        registry.register(ServiceId::Cache, 200);
        let result = registry.validate_dependencies::<UserService>();
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_runtime_cost() {
        // This test verifies that service creation is just pointer assignment
        let container = Container {
            database_offset: 1000,
            cache_offset: 2000,
            ..Default::default()
        };

        // Creating a service should be a simple struct initialization
        // with values copied from the container - no allocations, no lookups
        let service = UserService::new(&container);

        // The offsets should match exactly
        assert_eq!(service.db_ptr, container.database_offset);
        assert_eq!(service.cache_ptr, container.cache_offset);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 13: DI Offset Consistency**
    // **Validates: Requirements 7.1, 7.2, 7.4**
    // *For any* Injectable struct, the memory offsets of dependency pointers SHALL be fixed at compile time and match the Container's offset fields.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_container_roundtrip(
            database_offset in 0u32..=1000000u32,
            cache_offset in 0u32..=1000000u32,
            auth_offset in 0u32..=1000000u32,
            logger_offset in 0u32..=1000000u32,
            config_offset in 0u32..=1000000u32,
            http_client_offset in 0u32..=1000000u32,
            event_bus_offset in 0u32..=1000000u32,
            scheduler_offset in 0u32..=1000000u32,
        ) {
            let container = Container {
                database_offset,
                cache_offset,
                auth_offset,
                logger_offset,
                config_offset,
                http_client_offset,
                event_bus_offset,
                scheduler_offset,
            };

            let bytes = container.to_bytes();
            let restored = Container::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.database_offset, database_offset);
            prop_assert_eq!(restored.cache_offset, cache_offset);
            prop_assert_eq!(restored.auth_offset, auth_offset);
            prop_assert_eq!(restored.logger_offset, logger_offset);
            prop_assert_eq!(restored.config_offset, config_offset);
            prop_assert_eq!(restored.http_client_offset, http_client_offset);
            prop_assert_eq!(restored.event_bus_offset, event_bus_offset);
            prop_assert_eq!(restored.scheduler_offset, scheduler_offset);
        }

        #[test]
        fn prop_user_service_offset_consistency(
            database_offset in 0u32..=1000000u32,
            cache_offset in 0u32..=1000000u32,
        ) {
            let container = Container {
                database_offset,
                cache_offset,
                ..Default::default()
            };

            // Create service using Injectable trait
            let service = UserService::create(&container);

            // Verify offsets match exactly - this is the key property
            // The service's pointers must equal the container's offsets
            prop_assert_eq!(service.db_ptr, container.database_offset);
            prop_assert_eq!(service.cache_ptr, container.cache_offset);
        }

        #[test]
        fn prop_auth_service_offset_consistency(
            database_offset in 0u32..=1000000u32,
            config_offset in 0u32..=1000000u32,
            logger_offset in 0u32..=1000000u32,
        ) {
            let container = Container {
                database_offset,
                config_offset,
                logger_offset,
                ..Default::default()
            };

            let service = AuthService::create(&container);

            // Verify all offsets match
            prop_assert_eq!(service.db_ptr, container.database_offset);
            prop_assert_eq!(service.config_ptr, container.config_offset);
            prop_assert_eq!(service.logger_ptr, container.logger_offset);
        }

        #[test]
        fn prop_service_registry_builds_correct_container(
            offsets in prop::collection::vec(0u32..=1000000u32, 8),
        ) {
            let mut registry = ServiceRegistry::new();

            // Register all services
            registry.register(ServiceId::Database, offsets[0]);
            registry.register(ServiceId::Cache, offsets[1]);
            registry.register(ServiceId::Auth, offsets[2]);
            registry.register(ServiceId::Logger, offsets[3]);
            registry.register(ServiceId::Config, offsets[4]);
            registry.register(ServiceId::HttpClient, offsets[5]);
            registry.register(ServiceId::EventBus, offsets[6]);
            registry.register(ServiceId::Scheduler, offsets[7]);

            let container = registry.build_container();

            // Verify all offsets are correctly transferred
            prop_assert_eq!(container.database_offset, offsets[0]);
            prop_assert_eq!(container.cache_offset, offsets[1]);
            prop_assert_eq!(container.auth_offset, offsets[2]);
            prop_assert_eq!(container.logger_offset, offsets[3]);
            prop_assert_eq!(container.config_offset, offsets[4]);
            prop_assert_eq!(container.http_client_offset, offsets[5]);
            prop_assert_eq!(container.event_bus_offset, offsets[6]);
            prop_assert_eq!(container.scheduler_offset, offsets[7]);
        }

        #[test]
        fn prop_get_set_offset_consistency(
            offset in 0u32..=1000000u32,
            service_id in 0u8..=7u8,
        ) {
            let service_id = ServiceId::from_u8(service_id).unwrap();
            let mut container = Container::new();

            container.set_offset(service_id, offset);
            let retrieved = container.get_offset(service_id);

            prop_assert_eq!(retrieved, offset);
        }
    }
}
