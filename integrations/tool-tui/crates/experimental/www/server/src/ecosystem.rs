// Ecosystem integration for dx-server

#[cfg(feature = "query")]
use dx_www_query::QueryCache;

#[cfg(feature = "db")]
use dx_db::DbPool;

#[cfg(feature = "auth")]
use dx_www_auth::TokenGenerator;

#[cfg(feature = "sync")]
use dx_sync::ChannelManager;

use std::sync::Arc;

/// Cached query result type
pub type CachedResult = Vec<u8>;

/// Server ecosystem state
pub struct EcosystemState {
    #[cfg(feature = "query")]
    pub query_cache: Option<Arc<QueryCache<CachedResult>>>,

    #[cfg(feature = "db")]
    pub db_pool: Option<Arc<DbPool>>,

    #[cfg(feature = "auth")]
    pub token_generator: Option<Arc<TokenGenerator>>,

    #[cfg(feature = "sync")]
    pub channel_manager: Option<Arc<ChannelManager>>,
}

impl EcosystemState {
    /// Initialize ecosystem with enabled features
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "query")]
            query_cache: Some(Arc::new(QueryCache::<CachedResult>::new(1000))),

            #[cfg(feature = "db")]
            db_pool: None, // Initialized later with config

            #[cfg(feature = "auth")]
            token_generator: None, // Initialized with keypair

            #[cfg(feature = "sync")]
            channel_manager: Some(Arc::new(ChannelManager::new(1000))),
        }
    }
}

impl Default for EcosystemState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecosystem_state_creation() {
        let state = EcosystemState::new();

        #[cfg(feature = "query")]
        assert!(state.query_cache.is_some());

        #[cfg(feature = "sync")]
        assert!(state.channel_manager.is_some());
    }
}
