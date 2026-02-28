//! Resource Manager for DX Forge
//!
//! Provides centralized resource management with:
//! - Semaphore-based file handle limiting
//! - RAII wrapper for automatic handle release
//! - Operation queuing when at limit
//! - Graceful shutdown with timeout
//!
//! # Example
//! ```rust,ignore
//! use dx_forge::resource_manager::ResourceManager;
//!
//! let manager = ResourceManager::new(100);
//! let guard = manager.acquire_handle().await?;
//! // Use the handle...
//! // Guard automatically releases on drop
//! ```

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast};

/// Resource manager for controlling concurrent resource usage
pub struct ResourceManager {
    /// Semaphore for limiting concurrent handles
    semaphore: Arc<Semaphore>,
    /// Maximum number of handles allowed
    max_handles: usize,
    /// Current number of active handles
    active_handles: AtomicUsize,
    /// Number of operations waiting in queue
    queued_operations: AtomicUsize,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    /// Whether shutdown has been initiated
    shutdown_initiated: AtomicBool,
}

impl ResourceManager {
    /// Create a new resource manager with the specified maximum handles
    pub fn new(max_handles: usize) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            semaphore: Arc::new(Semaphore::new(max_handles)),
            max_handles,
            active_handles: AtomicUsize::new(0),
            queued_operations: AtomicUsize::new(0),
            shutdown_tx,
            shutdown_initiated: AtomicBool::new(false),
        }
    }

    /// Create a resource manager with default settings (1024 handles)
    pub fn with_defaults() -> Self {
        Self::new(1024)
    }

    /// Acquire a handle, waiting if necessary
    ///
    /// Returns a HandleGuard that automatically releases the handle on drop.
    /// If shutdown has been initiated, returns an error.
    pub async fn acquire_handle(&self) -> Result<HandleGuard<'_>> {
        if self.shutdown_initiated.load(Ordering::SeqCst) {
            return Err(anyhow!("Resource manager is shutting down"));
        }

        // Track queued operations
        self.queued_operations.fetch_add(1, Ordering::SeqCst);

        // Acquire permit from semaphore
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| anyhow!("Semaphore closed"))?;

        // No longer queued
        self.queued_operations.fetch_sub(1, Ordering::SeqCst);

        // Track active handles
        self.active_handles.fetch_add(1, Ordering::SeqCst);

        Ok(HandleGuard {
            _permit: permit,
            active_handles: &self.active_handles,
        })
    }

    /// Try to acquire a handle without waiting
    ///
    /// Returns None if no handles are available.
    pub fn try_acquire_handle(&self) -> Option<HandleGuard<'_>> {
        if self.shutdown_initiated.load(Ordering::SeqCst) {
            return None;
        }

        let permit = self.semaphore.clone().try_acquire_owned().ok()?;
        self.active_handles.fetch_add(1, Ordering::SeqCst);

        Some(HandleGuard {
            _permit: permit,
            active_handles: &self.active_handles,
        })
    }

    /// Acquire a handle with a timeout
    pub async fn acquire_handle_timeout(&self, timeout: Duration) -> Result<HandleGuard<'_>> {
        if self.shutdown_initiated.load(Ordering::SeqCst) {
            return Err(anyhow!("Resource manager is shutting down"));
        }

        self.queued_operations.fetch_add(1, Ordering::SeqCst);

        let result = tokio::time::timeout(timeout, self.semaphore.clone().acquire_owned()).await;

        self.queued_operations.fetch_sub(1, Ordering::SeqCst);

        match result {
            Ok(Ok(permit)) => {
                self.active_handles.fetch_add(1, Ordering::SeqCst);
                Ok(HandleGuard {
                    _permit: permit,
                    active_handles: &self.active_handles,
                })
            }
            Ok(Err(_)) => Err(anyhow!("Semaphore closed")),
            Err(_) => Err(anyhow!("Timeout waiting for handle")),
        }
    }

    /// Get the number of currently active handles
    pub fn active_handles(&self) -> usize {
        self.active_handles.load(Ordering::SeqCst)
    }

    /// Get the number of operations waiting in queue
    pub fn queued_operations(&self) -> usize {
        self.queued_operations.load(Ordering::SeqCst)
    }

    /// Get the maximum number of handles
    pub fn max_handles(&self) -> usize {
        self.max_handles
    }

    /// Get the number of available handles
    pub fn available_handles(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Check if the manager is at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.available_handles() == 0
    }

    /// Subscribe to shutdown notifications
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Initiate graceful shutdown
    ///
    /// Waits for all active handles to be released, up to the specified timeout.
    /// Returns Ok(()) if all handles were released, or an error if timeout was reached.
    pub async fn shutdown(&self, timeout: Duration) -> Result<()> {
        // Mark shutdown as initiated
        self.shutdown_initiated.store(true, Ordering::SeqCst);

        // Notify all subscribers
        let _ = self.shutdown_tx.send(());

        let deadline = Instant::now() + timeout;

        // Wait for all handles to be released
        while self.active_handles.load(Ordering::SeqCst) > 0 {
            if Instant::now() > deadline {
                let remaining = self.active_handles.load(Ordering::SeqCst);
                tracing::warn!("Shutdown timeout reached, {} handles still active", remaining);
                return Err(anyhow!("Shutdown timeout: {} handles still active", remaining));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        tracing::info!("Resource manager shutdown complete");
        Ok(())
    }

    /// Force shutdown without waiting
    pub fn force_shutdown(&self) {
        self.shutdown_initiated.store(true, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(());
        self.semaphore.close();
    }

    /// Check if shutdown has been initiated
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_initiated.load(Ordering::SeqCst)
    }
}

/// RAII guard for automatic handle release
pub struct HandleGuard<'a> {
    _permit: OwnedSemaphorePermit,
    active_handles: &'a AtomicUsize,
}

impl<'a> Drop for HandleGuard<'a> {
    fn drop(&mut self) {
        self.active_handles.fetch_sub(1, Ordering::SeqCst);
    }
}

impl<'a> HandleGuard<'a> {
    /// Check if this guard is still valid (not dropped)
    pub fn is_valid(&self) -> bool {
        true // If we have the guard, it's valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_and_release() {
        let manager = ResourceManager::new(10);

        assert_eq!(manager.active_handles(), 0);
        assert_eq!(manager.available_handles(), 10);

        let guard = manager.acquire_handle().await.unwrap();
        assert_eq!(manager.active_handles(), 1);
        assert_eq!(manager.available_handles(), 9);

        drop(guard);
        assert_eq!(manager.active_handles(), 0);
        assert_eq!(manager.available_handles(), 10);
    }

    #[tokio::test]
    async fn test_try_acquire() {
        let manager = ResourceManager::new(1);

        let guard1 = manager.try_acquire_handle();
        assert!(guard1.is_some());
        assert_eq!(manager.active_handles(), 1);

        let guard2 = manager.try_acquire_handle();
        assert!(guard2.is_none());

        drop(guard1);
        let guard3 = manager.try_acquire_handle();
        assert!(guard3.is_some());
    }

    #[tokio::test]
    async fn test_acquire_timeout() {
        let manager = ResourceManager::new(1);

        let _guard = manager.acquire_handle().await.unwrap();

        let result = manager.acquire_handle_timeout(Duration::from_millis(50)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let manager = ResourceManager::new(10);

        let result = manager.shutdown(Duration::from_millis(100)).await;
        assert!(result.is_ok());
        assert!(manager.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_with_active_handles() {
        let manager = Arc::new(ResourceManager::new(10));

        let guard = manager.acquire_handle().await.unwrap();

        let manager_clone = Arc::clone(&manager);
        let shutdown_handle =
            tokio::spawn(async move { manager_clone.shutdown(Duration::from_millis(200)).await });

        // Wait a bit then release
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(guard);

        let result = shutdown_handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_timeout() {
        let manager = ResourceManager::new(10);

        let _guard = manager.acquire_handle().await.unwrap();

        let result = manager.shutdown(Duration::from_millis(50)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_acquire_after_shutdown() {
        let manager = ResourceManager::new(10);

        manager.shutdown(Duration::from_millis(100)).await.unwrap();

        let result = manager.acquire_handle().await;
        assert!(result.is_err());
    }
}

/// Property-based tests for resource manager
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Arc;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 12: File Handle Limiting
        /// For any sequence of file operations, the number of concurrently held
        /// file handles SHALL never exceed the configured maximum limit.
        #[test]
        fn prop_handle_limiting(max_handles in 1..50usize, num_acquires in 1..100usize) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = Arc::new(ResourceManager::new(max_handles));
                let mut guards = Vec::new();

                for _ in 0..num_acquires {
                    if let Some(guard) = manager.try_acquire_handle() {
                        guards.push(guard);
                    }

                    // Invariant: active handles never exceed max
                    prop_assert!(manager.active_handles() <= max_handles);
                }

                // At capacity, we should have exactly max_handles
                prop_assert!(guards.len() <= max_handles);
                prop_assert_eq!(manager.active_handles(), guards.len());

                Ok(())
            })?;
        }
    }

    /// Property 13: Handle Queuing at Limit
    /// For any operation requested when the file handle limit is reached,
    /// the operation SHALL be queued and SHALL complete successfully once
    /// a handle becomes available.
    #[tokio::test]
    async fn prop_handle_queuing() {
        let manager = Arc::new(ResourceManager::new(2));

        // Acquire all handles
        let guard1 = manager.acquire_handle().await.unwrap();
        let guard2 = manager.acquire_handle().await.unwrap();

        assert_eq!(manager.active_handles(), 2);
        assert_eq!(manager.available_handles(), 0);

        // Try to acquire should fail immediately (no waiting)
        let try_result = manager.try_acquire_handle();
        assert!(try_result.is_none(), "Should not be able to acquire when at limit");

        // Release one handle
        drop(guard1);

        // Now we should be able to acquire
        let guard3 = manager.acquire_handle().await.unwrap();
        assert_eq!(manager.active_handles(), 2);

        // Clean up
        drop(guard2);
        drop(guard3);

        assert_eq!(manager.active_handles(), 0);
    }

    /// Property 13 (continued): Test queuing with timeout
    #[tokio::test]
    async fn prop_handle_queuing_with_timeout() {
        let manager = Arc::new(ResourceManager::new(1));

        // Acquire the only handle
        let guard1 = manager.acquire_handle().await.unwrap();
        assert_eq!(manager.active_handles(), 1);

        // Try to acquire with short timeout should fail
        let result = manager.acquire_handle_timeout(Duration::from_millis(10)).await;
        assert!(result.is_err(), "Should timeout when no handles available");

        // Release the handle
        drop(guard1);

        // Now acquire with timeout should succeed
        let guard2 = manager.acquire_handle_timeout(Duration::from_millis(100)).await;
        assert!(guard2.is_ok(), "Should succeed after handle released");
    }

    /// Test concurrent access to resource manager
    #[tokio::test]
    async fn test_concurrent_access() {
        let manager = Arc::new(ResourceManager::new(10));
        let mut handles = Vec::new();

        // Spawn many concurrent tasks
        for _ in 0..50 {
            let manager_clone = Arc::clone(&manager);
            handles.push(tokio::spawn(async move {
                let guard = manager_clone.acquire_handle().await.unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
                drop(guard);
            }));
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // All handles should be released
        assert_eq!(manager.active_handles(), 0);
        assert_eq!(manager.available_handles(), 10);
    }
}
