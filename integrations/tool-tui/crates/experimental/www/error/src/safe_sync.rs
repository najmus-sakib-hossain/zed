//! Safe synchronization primitives that recover from poisoning.
//!
//! These wrappers provide graceful recovery when a thread panics while holding a lock,
//! instead of propagating the panic to all subsequent lock attempts.

use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Error type for lock operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Lock acquisition failed due to an unrecoverable error
    AcquisitionFailed(String),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::AcquisitionFailed(msg) => write!(f, "Lock acquisition failed: {}", msg),
        }
    }
}

impl std::error::Error for LockError {}

/// A mutex wrapper that handles poisoning gracefully.
///
/// When a thread panics while holding a standard `Mutex`, the mutex becomes "poisoned"
/// and all subsequent lock attempts will fail. `SafeMutex` recovers from this state
/// by logging the event and returning the inner value anyway.
///
/// # Example
/// ```
/// use dx_www_error::SafeMutex;
///
/// let mutex = SafeMutex::new(42);
/// {
///     let mut guard = mutex.lock().unwrap();
///     *guard = 100;
/// }
/// assert_eq!(*mutex.lock().unwrap(), 100);
/// ```
pub struct SafeMutex<T> {
    inner: Mutex<T>,
}

impl<T> SafeMutex<T> {
    /// Create a new SafeMutex with the given value.
    pub fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(value),
        }
    }

    /// Lock the mutex, recovering from poisoning if necessary.
    ///
    /// If the mutex was poisoned by a panicking thread, this method will:
    /// 1. Log a warning about the poisoning
    /// 2. Recover the inner value
    /// 3. Return a valid guard
    ///
    /// This ensures that a panic in one thread doesn't cascade to crash
    /// all other threads that need access to the shared data.
    pub fn lock(&self) -> Result<SafeMutexGuard<'_, T>, LockError> {
        match self.inner.lock() {
            Ok(guard) => Ok(SafeMutexGuard {
                guard,
                was_poisoned: false,
            }),
            Err(poisoned) => {
                tracing::warn!("Mutex was poisoned, recovering inner value");
                Ok(SafeMutexGuard {
                    guard: poisoned.into_inner(),
                    was_poisoned: true,
                })
            }
        }
    }

    /// Try to lock the mutex without blocking.
    ///
    /// Returns `None` if the lock is currently held by another thread.
    /// Recovers from poisoning the same way as `lock()`.
    pub fn try_lock(&self) -> Option<SafeMutexGuard<'_, T>> {
        match self.inner.try_lock() {
            Ok(guard) => Some(SafeMutexGuard {
                guard,
                was_poisoned: false,
            }),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                tracing::warn!("Mutex was poisoned, recovering inner value");
                Some(SafeMutexGuard {
                    guard: poisoned.into_inner(),
                    was_poisoned: true,
                })
            }
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    /// Consume the mutex and return the inner value.
    ///
    /// Recovers from poisoning if necessary.
    pub fn into_inner(self) -> T {
        match self.inner.into_inner() {
            Ok(value) => value,
            Err(poisoned) => {
                tracing::warn!("Mutex was poisoned, recovering inner value");
                poisoned.into_inner()
            }
        }
    }

    /// Get a mutable reference to the inner value.
    ///
    /// This is safe because we have exclusive access to the SafeMutex.
    pub fn get_mut(&mut self) -> &mut T {
        match self.inner.get_mut() {
            Ok(value) => value,
            Err(poisoned) => {
                tracing::warn!("Mutex was poisoned, recovering inner value");
                poisoned.into_inner()
            }
        }
    }
}

impl<T: Default> Default for SafeMutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for SafeMutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.try_lock() {
            Ok(guard) => f.debug_struct("SafeMutex").field("data", &*guard).finish(),
            Err(_) => f.debug_struct("SafeMutex").field("data", &"<locked>").finish(),
        }
    }
}

/// Guard returned by `SafeMutex::lock()`.
///
/// Provides access to the protected data and tracks whether the mutex was poisoned.
pub struct SafeMutexGuard<'a, T> {
    guard: MutexGuard<'a, T>,
    was_poisoned: bool,
}

impl<'a, T> SafeMutexGuard<'a, T> {
    /// Returns true if the mutex was poisoned and recovered.
    pub fn was_poisoned(&self) -> bool {
        self.was_poisoned
    }
}

impl<'a, T> Deref for SafeMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> DerefMut for SafeMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

/// A read-write lock wrapper that handles poisoning gracefully.
///
/// Similar to `SafeMutex`, but allows multiple readers or a single writer.
/// Recovers from poisoning on both read and write lock attempts.
pub struct SafeRwLock<T> {
    inner: RwLock<T>,
}

impl<T> SafeRwLock<T> {
    /// Create a new SafeRwLock with the given value.
    pub fn new(value: T) -> Self {
        Self {
            inner: RwLock::new(value),
        }
    }

    /// Acquire a read lock, recovering from poisoning if necessary.
    pub fn read(&self) -> Result<SafeRwLockReadGuard<'_, T>, LockError> {
        match self.inner.read() {
            Ok(guard) => Ok(SafeRwLockReadGuard {
                guard,
                was_poisoned: false,
            }),
            Err(poisoned) => {
                tracing::warn!("RwLock was poisoned (read), recovering inner value");
                Ok(SafeRwLockReadGuard {
                    guard: poisoned.into_inner(),
                    was_poisoned: true,
                })
            }
        }
    }

    /// Acquire a write lock, recovering from poisoning if necessary.
    pub fn write(&self) -> Result<SafeRwLockWriteGuard<'_, T>, LockError> {
        match self.inner.write() {
            Ok(guard) => Ok(SafeRwLockWriteGuard {
                guard,
                was_poisoned: false,
            }),
            Err(poisoned) => {
                tracing::warn!("RwLock was poisoned (write), recovering inner value");
                Ok(SafeRwLockWriteGuard {
                    guard: poisoned.into_inner(),
                    was_poisoned: true,
                })
            }
        }
    }

    /// Try to acquire a read lock without blocking.
    pub fn try_read(&self) -> Option<SafeRwLockReadGuard<'_, T>> {
        match self.inner.try_read() {
            Ok(guard) => Some(SafeRwLockReadGuard {
                guard,
                was_poisoned: false,
            }),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                tracing::warn!("RwLock was poisoned (read), recovering inner value");
                Some(SafeRwLockReadGuard {
                    guard: poisoned.into_inner(),
                    was_poisoned: true,
                })
            }
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    /// Try to acquire a write lock without blocking.
    pub fn try_write(&self) -> Option<SafeRwLockWriteGuard<'_, T>> {
        match self.inner.try_write() {
            Ok(guard) => Some(SafeRwLockWriteGuard {
                guard,
                was_poisoned: false,
            }),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                tracing::warn!("RwLock was poisoned (write), recovering inner value");
                Some(SafeRwLockWriteGuard {
                    guard: poisoned.into_inner(),
                    was_poisoned: true,
                })
            }
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    /// Consume the lock and return the inner value.
    pub fn into_inner(self) -> T {
        match self.inner.into_inner() {
            Ok(value) => value,
            Err(poisoned) => {
                tracing::warn!("RwLock was poisoned, recovering inner value");
                poisoned.into_inner()
            }
        }
    }

    /// Get a mutable reference to the inner value.
    pub fn get_mut(&mut self) -> &mut T {
        match self.inner.get_mut() {
            Ok(value) => value,
            Err(poisoned) => {
                tracing::warn!("RwLock was poisoned, recovering inner value");
                poisoned.into_inner()
            }
        }
    }
}

impl<T: Default> Default for SafeRwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for SafeRwLock<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.try_read() {
            Ok(guard) => f.debug_struct("SafeRwLock").field("data", &*guard).finish(),
            Err(_) => f.debug_struct("SafeRwLock").field("data", &"<locked>").finish(),
        }
    }
}

/// Guard returned by `SafeRwLock::read()`.
pub struct SafeRwLockReadGuard<'a, T> {
    guard: RwLockReadGuard<'a, T>,
    was_poisoned: bool,
}

impl<'a, T> SafeRwLockReadGuard<'a, T> {
    /// Returns true if the lock was poisoned and recovered.
    pub fn was_poisoned(&self) -> bool {
        self.was_poisoned
    }
}

impl<'a, T> Deref for SafeRwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

/// Guard returned by `SafeRwLock::write()`.
pub struct SafeRwLockWriteGuard<'a, T> {
    guard: RwLockWriteGuard<'a, T>,
    was_poisoned: bool,
}

impl<'a, T> SafeRwLockWriteGuard<'a, T> {
    /// Returns true if the lock was poisoned and recovered.
    pub fn was_poisoned(&self) -> bool {
        self.was_poisoned
    }
}

impl<'a, T> Deref for SafeRwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> DerefMut for SafeRwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_safe_mutex_basic() {
        let mutex = SafeMutex::new(42);
        {
            let mut guard = mutex.lock().unwrap();
            assert_eq!(*guard, 42);
            *guard = 100;
        }
        assert_eq!(*mutex.lock().unwrap(), 100);
    }

    #[test]
    fn test_safe_mutex_try_lock() {
        let mutex = SafeMutex::new(42);
        let guard = mutex.try_lock();
        assert!(guard.is_some());
        assert_eq!(*guard.unwrap(), 42);
    }

    #[test]
    fn test_safe_mutex_into_inner() {
        let mutex = SafeMutex::new(42);
        assert_eq!(mutex.into_inner(), 42);
    }

    #[test]
    fn test_safe_mutex_get_mut() {
        let mut mutex = SafeMutex::new(42);
        *mutex.get_mut() = 100;
        assert_eq!(*mutex.lock().unwrap(), 100);
    }

    #[test]
    fn test_safe_mutex_concurrent() {
        let mutex = Arc::new(SafeMutex::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let mutex_clone = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut guard = mutex_clone.lock().unwrap();
                    *guard += 1;
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(*mutex.lock().unwrap(), 1000);
    }

    #[test]
    fn test_safe_rwlock_basic() {
        let lock = SafeRwLock::new(42);

        // Multiple readers
        {
            let guard1 = lock.read().unwrap();
            let guard2 = lock.read().unwrap();
            assert_eq!(*guard1, 42);
            assert_eq!(*guard2, 42);
        }

        // Single writer
        {
            let mut guard = lock.write().unwrap();
            *guard = 100;
        }

        assert_eq!(*lock.read().unwrap(), 100);
    }

    #[test]
    fn test_safe_rwlock_try_read() {
        let lock = SafeRwLock::new(42);
        let guard = lock.try_read();
        assert!(guard.is_some());
        assert_eq!(*guard.unwrap(), 42);
    }

    #[test]
    fn test_safe_rwlock_try_write() {
        let lock = SafeRwLock::new(42);
        let guard = lock.try_write();
        assert!(guard.is_some());
    }

    #[test]
    fn test_safe_rwlock_into_inner() {
        let lock = SafeRwLock::new(42);
        assert_eq!(lock.into_inner(), 42);
    }

    #[test]
    fn test_safe_rwlock_get_mut() {
        let mut lock = SafeRwLock::new(42);
        *lock.get_mut() = 100;
        assert_eq!(*lock.read().unwrap(), 100);
    }

    #[test]
    fn test_safe_mutex_default() {
        let mutex: SafeMutex<i32> = SafeMutex::default();
        assert_eq!(*mutex.lock().unwrap(), 0);
    }

    #[test]
    fn test_safe_rwlock_default() {
        let lock: SafeRwLock<i32> = SafeRwLock::default();
        assert_eq!(*lock.read().unwrap(), 0);
    }
}
