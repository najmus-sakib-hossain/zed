//! Python async/await compatible future

use atomic_waker::AtomicWaker;
use parking_lot::Mutex;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// State of a PyFuture.
enum FutureState<T> {
    /// Operation is still pending
    Pending,
    /// Operation completed successfully
    Ready(T),
    /// Operation failed with an error
    Error(io::Error),
}

/// A future that can be used with Python's async/await.
///
/// This future is designed to integrate with the reactor's completion
/// mechanism, allowing async I/O operations to be awaited.
///
/// # Example
///
/// ```ignore
/// use dx_py_reactor::PyFuture;
///
/// async fn read_file(reactor: &mut impl Reactor, fd: RawFd) -> io::Result<Vec<u8>> {
///     let future = PyFuture::new();
///     let future_clone = future.clone();
///
///     // Submit read operation with callback
///     reactor.submit_with_callback(
///         IoOperation::Read { fd, buf, offset: 0, user_data: 1 },
///         move |result| {
///             match result {
///                 Ok(bytes) => future_clone.set_result(bytes),
///                 Err(e) => future_clone.set_error(e),
///             }
///         }
///     )?;
///
///     future.await
/// }
/// ```
pub struct PyFuture<T> {
    inner: Arc<PyFutureInner<T>>,
}

struct PyFutureInner<T> {
    state: Mutex<FutureState<T>>,
    waker: AtomicWaker,
}

impl<T> PyFuture<T> {
    /// Create a new pending future.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(PyFutureInner {
                state: Mutex::new(FutureState::Pending),
                waker: AtomicWaker::new(),
            }),
        }
    }

    /// Set the result of the future, waking any waiting tasks.
    pub fn set_result(&self, result: T) {
        let mut state = self.inner.state.lock();
        *state = FutureState::Ready(result);
        drop(state);
        self.inner.waker.wake();
    }

    /// Set an error on the future, waking any waiting tasks.
    pub fn set_error(&self, error: io::Error) {
        let mut state = self.inner.state.lock();
        *state = FutureState::Error(error);
        drop(state);
        self.inner.waker.wake();
    }

    /// Check if the future is still pending.
    pub fn is_pending(&self) -> bool {
        matches!(*self.inner.state.lock(), FutureState::Pending)
    }

    /// Check if the future is ready (completed or errored).
    pub fn is_ready(&self) -> bool {
        !self.is_pending()
    }

    /// Try to get the result without blocking.
    /// Returns None if the future is still pending.
    pub fn try_get(&self) -> Option<io::Result<T>>
    where
        T: Clone,
    {
        let state = self.inner.state.lock();
        match &*state {
            FutureState::Pending => None,
            FutureState::Ready(v) => Some(Ok(v.clone())),
            FutureState::Error(e) => Some(Err(io::Error::new(e.kind(), e.to_string()))),
        }
    }
}

impl<T> Default for PyFuture<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for PyFuture<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Future for PyFuture<T> {
    type Output = io::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Register the waker first
        self.inner.waker.register(cx.waker());

        // Check the state
        let mut state = self.inner.state.lock();

        // Take the state, replacing with Pending
        let current = std::mem::replace(&mut *state, FutureState::Pending);

        match current {
            FutureState::Pending => {
                // Still pending, put it back
                *state = FutureState::Pending;
                Poll::Pending
            }
            FutureState::Ready(value) => Poll::Ready(Ok(value)),
            FutureState::Error(err) => Poll::Ready(Err(err)),
        }
    }
}

/// A future that completes when multiple operations complete.
pub struct PyFutureAll<T> {
    futures: Vec<PyFuture<T>>,
}

impl<T: Clone> PyFutureAll<T> {
    /// Create a future that completes when all inner futures complete.
    pub fn new(futures: Vec<PyFuture<T>>) -> Self {
        Self { futures }
    }

    /// Check if all futures are ready.
    pub fn all_ready(&self) -> bool {
        self.futures.iter().all(|f| f.is_ready())
    }

    /// Try to get all results without blocking.
    pub fn try_get_all(&self) -> Option<Vec<io::Result<T>>> {
        if self.all_ready() {
            Some(self.futures.iter().filter_map(|f| f.try_get()).collect())
        } else {
            None
        }
    }
}

impl<T: Clone> Future for PyFutureAll<T> {
    type Output = Vec<io::Result<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_ready = true;

        for future in &self.futures {
            // Register waker for each future
            future.inner.waker.register(cx.waker());

            if future.is_pending() {
                all_ready = false;
            }
        }

        if all_ready {
            let results = self.futures.iter().filter_map(|f| f.try_get()).collect();
            Poll::Ready(results)
        } else {
            Poll::Pending
        }
    }
}

/// A future that completes when any of the inner futures complete.
pub struct PyFutureAny<T> {
    futures: Vec<PyFuture<T>>,
}

impl<T: Clone> PyFutureAny<T> {
    /// Create a future that completes when any inner future completes.
    pub fn new(futures: Vec<PyFuture<T>>) -> Self {
        Self { futures }
    }

    /// Check if any future is ready.
    pub fn any_ready(&self) -> bool {
        self.futures.iter().any(|f| f.is_ready())
    }

    /// Get the index and result of the first ready future.
    pub fn try_get_first(&self) -> Option<(usize, io::Result<T>)> {
        for (i, f) in self.futures.iter().enumerate() {
            if let Some(result) = f.try_get() {
                return Some((i, result));
            }
        }
        None
    }
}

impl<T: Clone> Future for PyFutureAny<T> {
    type Output = (usize, io::Result<T>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        for (i, future) in self.futures.iter().enumerate() {
            // Register waker for each future
            future.inner.waker.register(cx.waker());

            if let Some(result) = future.try_get() {
                return Poll::Ready((i, result));
            }
        }

        Poll::Pending
    }
}

/// A callback-based completion handler.
pub type CompletionCallback = Box<dyn FnOnce(io::Result<usize>) + Send + 'static>;

/// A registry for mapping user_data to callbacks.
pub struct CallbackRegistry {
    callbacks: dashmap::DashMap<u64, CompletionCallback>,
    next_id: std::sync::atomic::AtomicU64,
}

impl CallbackRegistry {
    /// Create a new callback registry.
    pub fn new() -> Self {
        Self {
            callbacks: dashmap::DashMap::new(),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Register a callback and return its user_data.
    pub fn register<F>(&self, callback: F) -> u64
    where
        F: FnOnce(io::Result<usize>) + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.callbacks.insert(id, Box::new(callback));
        id
    }

    /// Remove and return a callback by its user_data.
    pub fn remove(&self, user_data: u64) -> Option<CompletionCallback> {
        self.callbacks.remove(&user_data).map(|(_, cb)| cb)
    }

    /// Invoke a callback with the given result.
    pub fn invoke(&self, user_data: u64, result: io::Result<usize>) {
        if let Some(callback) = self.remove(user_data) {
            callback(result);
        }
    }

    /// Get the number of pending callbacks.
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    /// Check if there are no pending callbacks.
    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_future_set_result() {
        let future = PyFuture::new();
        assert!(future.is_pending());

        future.set_result(42);
        assert!(future.is_ready());
        assert_eq!(future.try_get().unwrap().unwrap(), 42);
    }

    #[test]
    fn test_py_future_set_error() {
        let future: PyFuture<i32> = PyFuture::new();
        future.set_error(io::Error::new(io::ErrorKind::NotFound, "not found"));

        assert!(future.is_ready());
        assert!(future.try_get().unwrap().is_err());
    }

    #[test]
    fn test_py_future_clone() {
        let future1 = PyFuture::new();
        let future2 = future1.clone();

        future1.set_result(42);

        // Both should see the result
        assert_eq!(future1.try_get().unwrap().unwrap(), 42);
        assert_eq!(future2.try_get().unwrap().unwrap(), 42);
    }

    #[test]
    fn test_callback_registry() {
        let registry = CallbackRegistry::new();
        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();

        let id = registry.register(move |r| {
            *result_clone.lock() = Some(r.unwrap());
        });

        assert_eq!(registry.len(), 1);

        registry.invoke(id, Ok(100));

        assert_eq!(registry.len(), 0);
        assert_eq!(*result.lock(), Some(100));
    }

    #[tokio::test]
    async fn test_py_future_await() {
        let future = PyFuture::new();
        let future_clone = future.clone();

        // Spawn a task to set the result
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            future_clone.set_result(42);
        });

        let result = future.await.unwrap();
        assert_eq!(result, 42);
    }
}
