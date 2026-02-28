//! Replay protection for DCP protocol.
//!
//! Provides nonce tracking and timestamp expiration checking
//! to prevent replay attacks.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::SecurityError;

/// Default expiration window (5 minutes)
const DEFAULT_EXPIRATION_SECS: u64 = 300;

/// Default maximum nonces to track
const DEFAULT_MAX_NONCES: usize = 10000;

/// Nonce store for replay protection
pub struct NonceStore {
    /// Seen nonces with their timestamps
    nonces: HashMap<u64, u64>,
    /// Maximum number of nonces to track
    max_nonces: usize,
    /// Expiration window in seconds
    expiration_secs: u64,
}

impl NonceStore {
    /// Create a new nonce store with default settings
    pub fn new() -> Self {
        Self {
            nonces: HashMap::new(),
            max_nonces: DEFAULT_MAX_NONCES,
            expiration_secs: DEFAULT_EXPIRATION_SECS,
        }
    }

    /// Create a nonce store with custom settings
    pub fn with_config(max_nonces: usize, expiration_secs: u64) -> Self {
        Self {
            nonces: HashMap::with_capacity(max_nonces),
            max_nonces,
            expiration_secs,
        }
    }

    /// Get the current timestamp in seconds since UNIX epoch
    pub fn current_timestamp() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs()
    }

    /// Check if a timestamp is expired
    pub fn is_expired(&self, timestamp: u64) -> bool {
        let now = Self::current_timestamp();
        if timestamp > now {
            // Future timestamp - allow some clock skew (60 seconds)
            return timestamp > now + 60;
        }
        now - timestamp > self.expiration_secs
    }

    /// Check and record a nonce
    /// Returns Ok(()) if the nonce is valid and not seen before
    /// Returns Err(ReplayAttack) if the nonce was already used
    /// Returns Err(ExpiredTimestamp) if the timestamp is too old
    pub fn check_nonce(&mut self, nonce: u64, timestamp: u64) -> Result<(), SecurityError> {
        // Check timestamp first
        if self.is_expired(timestamp) {
            return Err(SecurityError::ExpiredTimestamp);
        }

        // Check if nonce was already used
        if self.nonces.contains_key(&nonce) {
            return Err(SecurityError::ReplayAttack);
        }

        // Clean up old nonces if we're at capacity
        if self.nonces.len() >= self.max_nonces {
            self.cleanup_expired();
        }

        // Record the nonce
        self.nonces.insert(nonce, timestamp);
        Ok(())
    }

    /// Remove expired nonces from the store
    pub fn cleanup_expired(&mut self) {
        let now = Self::current_timestamp();
        self.nonces.retain(|_, &mut ts| now.saturating_sub(ts) <= self.expiration_secs);
    }

    /// Get the number of tracked nonces
    pub fn len(&self) -> usize {
        self.nonces.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.nonces.is_empty()
    }

    /// Clear all tracked nonces
    pub fn clear(&mut self) {
        self.nonces.clear();
    }
}

impl Default for NonceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_store_basic() {
        let mut store = NonceStore::new();
        let now = NonceStore::current_timestamp();

        // First use should succeed
        assert!(store.check_nonce(12345, now).is_ok());
        assert_eq!(store.len(), 1);

        // Reuse should fail
        assert_eq!(store.check_nonce(12345, now), Err(SecurityError::ReplayAttack));

        // Different nonce should succeed
        assert!(store.check_nonce(67890, now).is_ok());
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_expired_timestamp() {
        let mut store = NonceStore::with_config(100, 60); // 60 second expiration
        let now = NonceStore::current_timestamp();

        // Current timestamp should work
        assert!(store.check_nonce(1, now).is_ok());

        // Old timestamp should fail
        let old = now.saturating_sub(120); // 2 minutes ago
        assert_eq!(store.check_nonce(2, old), Err(SecurityError::ExpiredTimestamp));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut store = NonceStore::with_config(100, 1); // 1 second expiration
        let now = NonceStore::current_timestamp();

        // Add some nonces
        store.nonces.insert(1, now);
        store.nonces.insert(2, now.saturating_sub(10)); // Already expired
        store.nonces.insert(3, now.saturating_sub(10)); // Already expired

        store.cleanup_expired();

        // Only the recent one should remain
        assert_eq!(store.len(), 1);
        assert!(store.nonces.contains_key(&1));
    }

    #[test]
    fn test_future_timestamp() {
        let mut store = NonceStore::new();
        let now = NonceStore::current_timestamp();

        // Slightly in the future (within skew) should work
        assert!(store.check_nonce(1, now + 30).is_ok());

        // Far in the future should fail
        assert_eq!(store.check_nonce(2, now + 120), Err(SecurityError::ExpiredTimestamp));
    }

    #[test]
    fn test_clear() {
        let mut store = NonceStore::new();
        let now = NonceStore::current_timestamp();

        store.check_nonce(1, now).unwrap();
        store.check_nonce(2, now).unwrap();
        assert_eq!(store.len(), 2);

        store.clear();
        assert!(store.is_empty());

        // Can reuse nonces after clear
        assert!(store.check_nonce(1, now).is_ok());
    }
}
