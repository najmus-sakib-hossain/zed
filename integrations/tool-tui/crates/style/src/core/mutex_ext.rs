//! Safe mutex handling extension trait
//!
//! This module provides a `MutexExt` trait that offers safe mutex operations
//! with proper error handling for poisoned mutexes.
//!
//! # Requirements
//!
//! - 5.1: Handle PoisonError gracefully
//! - 5.2: Log poison events with tracing

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Extension trait for safe mutex operations
///
/// This trait provides methods for acquiring mutex locks with proper
/// error handling instead of panicking on poison.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Mutex;
/// use style::core::mutex_ext::MutexExt;
/// use style::StyleError;
///
/// let mutex = Mutex::new(42);
/// match mutex.lock_or_recover() {
///     Ok(guard) => println!("Value: {}", *guard),
///     Err(e) => eprintln!("Mutex poisoned: {}", e),
/// }
/// ```
pub trait MutexExt<T> {
    /// The error type returned when the mutex is poisoned
    type Error;

    /// Lock the mutex, returning an error if poisoned
    ///
    /// This method attempts to acquire the mutex lock. If the mutex is poisoned
    /// (another thread panicked while holding the lock), it logs the event
    /// and returns an error instead of panicking.
    ///
    /// # Returns
    ///
    /// - `Ok(MutexGuard)` if the lock was successfully acquired
    /// - `Err(Self::Error)` if the mutex was poisoned
    ///
    /// # Requirements
    ///
    /// - 5.1: Handle PoisonError gracefully
    /// - 5.2: Log poison events with tracing
    fn lock_or_recover(&self) -> Result<MutexGuard<'_, T>, Self::Error>;
}

/// Error type for mutex poisoning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutexPoisonedError;

impl std::fmt::Display for MutexPoisonedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mutex was poisoned - another thread panicked while holding the lock")
    }
}

impl std::error::Error for MutexPoisonedError {}

impl<T> MutexExt<T> for Mutex<T> {
    type Error = MutexPoisonedError;

    fn lock_or_recover(&self) -> Result<MutexGuard<'_, T>, Self::Error> {
        self.lock().map_err(|poisoned: PoisonError<MutexGuard<'_, T>>| {
            // Log the poison event with tracing
            tracing::error!(
                "Mutex was poisoned - another thread panicked while holding the lock. \
                 Recovery not possible, returning error."
            );

            // We could potentially recover by calling poisoned.into_inner()
            // but that's risky as the data may be in an inconsistent state.
            // For safety, we return an error and let the caller decide.
            let _ = poisoned; // Acknowledge we're not recovering

            MutexPoisonedError
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_lock_or_recover_success() {
        let mutex = Mutex::new(42);
        let result = mutex.lock_or_recover();
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), 42);
    }

    #[test]
    fn test_lock_or_recover_with_modification() {
        let mutex = Mutex::new(0);
        {
            let mut guard = mutex.lock_or_recover().unwrap();
            *guard = 100;
        }
        let guard = mutex.lock_or_recover().unwrap();
        assert_eq!(*guard, 100);
    }

    #[test]
    fn test_lock_or_recover_poisoned() {
        let mutex = Arc::new(Mutex::new(42));
        let mutex_clone = Arc::clone(&mutex);

        // Spawn a thread that will panic while holding the lock
        let handle = thread::spawn(move || {
            let _guard = mutex_clone.lock().unwrap();
            panic!("Intentional panic to poison the mutex");
        });

        // Wait for the thread to finish (it will panic)
        let _ = handle.join();

        // Now try to lock the poisoned mutex
        let result = mutex.lock_or_recover();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), MutexPoisonedError);
    }

    #[test]
    fn test_lock_or_recover_multiple_successful_locks() {
        let mutex = Mutex::new(vec![1, 2, 3]);

        // First lock
        {
            let mut guard = mutex.lock_or_recover().unwrap();
            guard.push(4);
        }

        // Second lock
        {
            let guard = mutex.lock_or_recover().unwrap();
            assert_eq!(*guard, vec![1, 2, 3, 4]);
        }

        // Third lock
        {
            let mut guard = mutex.lock_or_recover().unwrap();
            guard.clear();
        }

        // Verify final state
        let guard = mutex.lock_or_recover().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn test_mutex_poisoned_error_display() {
        let err = MutexPoisonedError;
        let display = format!("{}", err);
        assert!(display.contains("Mutex was poisoned"));
    }
}
