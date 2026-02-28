//! Atomic Synchronization
//!
//! Lock-free synchronization primitives for rule state.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Sync state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// Idle, no sync in progress
    Idle,
    /// Syncing in progress
    Syncing,
    /// Sync completed successfully
    Completed,
    /// Sync failed
    Failed,
}

/// Atomic sync controller
#[derive(Debug)]
pub struct AtomicSync {
    /// Current state (encoded as u8)
    state: AtomicU64,
    /// Sync in progress flag
    in_progress: AtomicBool,
    /// Last sync timestamp
    last_sync: AtomicU64,
    /// Sync count
    sync_count: AtomicU64,
    /// Error count
    error_count: AtomicU64,
}

impl AtomicSync {
    /// Create a new sync controller
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(SyncState::Idle as u64),
            in_progress: AtomicBool::new(false),
            last_sync: AtomicU64::new(0),
            sync_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }

    /// Get current state
    pub fn state(&self) -> SyncState {
        match self.state.load(Ordering::SeqCst) {
            0 => SyncState::Idle,
            1 => SyncState::Syncing,
            2 => SyncState::Completed,
            _ => SyncState::Failed,
        }
    }

    /// Try to start a sync (returns false if already syncing)
    pub fn try_start(&self) -> bool {
        if self
            .in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            {
                self.state.store(SyncState::Syncing as u64, Ordering::SeqCst);
                true
            }
        } else {
            false
        }
    }

    /// Mark sync as completed
    pub fn complete(&self) {
        self.state.store(SyncState::Completed as u64, Ordering::SeqCst);
        self.in_progress.store(false, Ordering::SeqCst);
        self.last_sync.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            Ordering::SeqCst,
        );
        self.sync_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark sync as failed
    pub fn fail(&self) {
        self.state.store(SyncState::Failed as u64, Ordering::SeqCst);
        self.in_progress.store(false, Ordering::SeqCst);
        self.error_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Reset to idle
    pub fn reset(&self) {
        self.state.store(SyncState::Idle as u64, Ordering::SeqCst);
        self.in_progress.store(false, Ordering::SeqCst);
    }

    /// Check if sync is in progress
    pub fn is_syncing(&self) -> bool {
        self.in_progress.load(Ordering::SeqCst)
    }

    /// Get last sync timestamp
    pub fn last_sync_time(&self) -> u64 {
        self.last_sync.load(Ordering::SeqCst)
    }

    /// Get sync count
    pub fn sync_count(&self) -> u64 {
        self.sync_count.load(Ordering::SeqCst)
    }

    /// Get error count
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::SeqCst)
    }

    /// Get time since last sync
    pub fn time_since_sync(&self) -> Option<u64> {
        let last = self.last_sync.load(Ordering::SeqCst);
        if last == 0 {
            return None;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Some(now.saturating_sub(last))
    }
}

impl Default for AtomicSync {
    fn default() -> Self {
        Self::new()
    }
}

/// Sync guard that auto-completes or fails on drop
pub struct SyncGuard<'a> {
    sync: &'a AtomicSync,
    completed: bool,
}

impl<'a> SyncGuard<'a> {
    /// Create a new sync guard
    pub fn new(sync: &'a AtomicSync) -> Option<Self> {
        if sync.try_start() {
            Some(Self {
                sync,
                completed: false,
            })
        } else {
            None
        }
    }

    /// Mark sync as completed
    pub fn complete(mut self) {
        self.completed = true;
        self.sync.complete();
    }

    /// Mark sync as failed
    pub fn fail(mut self) {
        self.completed = true;
        self.sync.fail();
    }
}

impl<'a> Drop for SyncGuard<'a> {
    fn drop(&mut self) {
        if !self.completed {
            self.sync.fail();
        }
    }
}

/// Thread-safe sync handle
pub type AtomicSyncHandle = Arc<AtomicSync>;

/// Create a new sync handle
pub fn create_sync_handle() -> AtomicSyncHandle {
    Arc::new(AtomicSync::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_lifecycle() {
        let sync = AtomicSync::new();

        assert_eq!(sync.state(), SyncState::Idle);
        assert!(!sync.is_syncing());

        assert!(sync.try_start());
        assert_eq!(sync.state(), SyncState::Syncing);
        assert!(sync.is_syncing());

        // Can't start while syncing
        assert!(!sync.try_start());

        sync.complete();
        assert_eq!(sync.state(), SyncState::Completed);
        assert!(!sync.is_syncing());
        assert_eq!(sync.sync_count(), 1);
    }

    #[test]
    fn test_sync_guard() {
        let sync = AtomicSync::new();

        {
            let guard = SyncGuard::new(&sync).unwrap();
            assert!(sync.is_syncing());
            guard.complete();
        }

        assert_eq!(sync.state(), SyncState::Completed);
    }

    #[test]
    fn test_sync_guard_auto_fail() {
        let sync = AtomicSync::new();

        {
            let _guard = SyncGuard::new(&sync).unwrap();
            assert!(sync.is_syncing());
            // Guard dropped without complete()
        }

        assert_eq!(sync.state(), SyncState::Failed);
        assert_eq!(sync.error_count(), 1);
    }

    #[test]
    fn test_thread_safety() {
        let sync = Arc::new(AtomicSync::new());

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let sync = sync.clone();
                std::thread::spawn(move || {
                    if sync.try_start() {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        sync.complete();
                        true
                    } else {
                        false
                    }
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let successes: usize = results.into_iter().filter(|&b| b).count();

        // Only one thread should have succeeded at a time
        assert!(successes >= 1);
    }
}
