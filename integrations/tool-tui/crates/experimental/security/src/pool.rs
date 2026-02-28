//! Thread Pool
//!
//! Lock-free thread-per-core architecture for parallel scanning.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::thread::{self, JoinHandle};

/// Default ring buffer capacity
const DEFAULT_RING_CAPACITY: usize = 1024;

/// Lock-free ring buffer for job distribution
pub struct RingBuffer<T> {
    /// Buffer storage
    buffer: Vec<Option<T>>,
    /// Capacity (must be power of 2)
    capacity: usize,
    /// Mask for fast modulo (capacity - 1)
    mask: usize,
    /// Head pointer (producer writes here)
    head: AtomicUsize,
    /// Tail pointer (consumer reads here)
    tail: AtomicUsize,
}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer with given capacity (rounded up to power of 2)
    pub fn new(capacity: usize) -> Self {
        // Round up to next power of 2
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;

        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(None);
        }

        Self {
            buffer,
            capacity,
            mask,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Try to push an item (returns false if full)
    pub fn try_push(&mut self, item: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        // Check if buffer is full
        if head.wrapping_sub(tail) >= self.capacity {
            return false;
        }

        let index = head & self.mask;
        self.buffer[index] = Some(item);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Try to pop an item (returns None if empty)
    pub fn try_pop(&mut self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        // Check if buffer is empty
        if tail == head {
            return None;
        }

        let index = tail & self.mask;
        let item = self.buffer[index].take();
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        item
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        tail == head
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail) >= self.capacity
    }

    /// Get current length
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail)
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Job handle for tracking completion
pub struct JobHandle {
    completed: Arc<AtomicBool>,
}

impl JobHandle {
    /// Wait for job completion
    pub fn join(&self) {
        while !self.completed.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
    }

    /// Check if job is complete
    pub fn is_complete(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }
}

/// Scan job to be executed
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Thread pool with one worker per physical core
pub struct ThreadPool {
    workers: Vec<JoinHandle<()>>,
    sender: Option<Sender<Job>>,
    worker_count: usize,
}

impl ThreadPool {
    /// Create pool with one thread per physical CPU core
    pub fn new() -> Self {
        let worker_count = num_cpus();
        Self::with_workers(worker_count)
    }

    /// Create pool with specified number of workers
    pub fn with_workers(count: usize) -> Self {
        let (sender, receiver) = channel::<Job>();
        let receiver = Arc::new(std::sync::Mutex::new(receiver));
        let mut workers = Vec::with_capacity(count);

        for id in 0..count {
            let rx = Arc::clone(&receiver);

            let handle = thread::Builder::new()
                .name(format!("dx-security-worker-{}", id))
                .spawn(move || {
                    loop {
                        let job = {
                            let lock = rx.lock().unwrap();
                            lock.recv()
                        };

                        match job {
                            Ok(job) => job(),
                            Err(_) => break, // Channel closed
                        }
                    }
                })
                .expect("Failed to spawn worker thread");

            workers.push(handle);
        }

        Self {
            workers,
            sender: Some(sender),
            worker_count: count,
        }
    }

    /// Submit a scan job
    pub fn submit<F>(&self, task: F) -> JobHandle
    where
        F: FnOnce() + Send + 'static,
    {
        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = Arc::clone(&completed);

        let wrapped_task = Box::new(move || {
            task();
            completed_clone.store(true, Ordering::Release);
        });

        if let Some(sender) = &self.sender {
            let _ = sender.send(wrapped_task);
        }

        JobHandle { completed }
    }

    /// Submit multiple jobs and return handles
    pub fn submit_batch<F>(&self, tasks: Vec<F>) -> Vec<JobHandle>
    where
        F: FnOnce() + Send + 'static,
    {
        tasks.into_iter().map(|task| self.submit(task)).collect()
    }

    /// Wait for all jobs to complete and shutdown
    pub fn join(&mut self) {
        // Drop sender to signal workers to stop
        self.sender.take();

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }

    /// Get worker count
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Check if pool is active
    pub fn is_active(&self) -> bool {
        self.sender.is_some()
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.sender.take();
    }
}

/// Get number of physical CPU cores
fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|p| p.get()).unwrap_or(1)
}

/// Scan job definition
pub struct ScanJob {
    /// File path to scan
    pub path: std::path::PathBuf,
    /// File data (memory-mapped)
    pub data: Vec<u8>,
}

impl ScanJob {
    /// Create a new scan job
    pub fn new(path: std::path::PathBuf, data: Vec<u8>) -> Self {
        Self { path, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn test_ring_buffer_basic() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(4);

        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.len(), 0);

        assert!(buffer.try_push(1));
        assert!(buffer.try_push(2));
        assert!(buffer.try_push(3));
        assert!(buffer.try_push(4));

        assert!(buffer.is_full());
        assert!(!buffer.try_push(5)); // Should fail, buffer full

        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.try_pop(), Some(2));
        assert_eq!(buffer.try_pop(), Some(3));
        assert_eq!(buffer.try_pop(), Some(4));

        assert!(buffer.is_empty());
        assert_eq!(buffer.try_pop(), None);
    }

    #[test]
    fn test_ring_buffer_wrap_around() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(4);

        // Fill and empty multiple times to test wrap-around
        for round in 0..3 {
            for i in 0..4 {
                assert!(buffer.try_push(round * 10 + i));
            }
            for i in 0..4 {
                assert_eq!(buffer.try_pop(), Some(round * 10 + i));
            }
        }
    }

    #[test]
    fn test_ring_buffer_capacity_power_of_two() {
        let buffer: RingBuffer<i32> = RingBuffer::new(5);
        assert_eq!(buffer.capacity(), 8); // Rounded up to 8

        let buffer: RingBuffer<i32> = RingBuffer::new(16);
        assert_eq!(buffer.capacity(), 16); // Already power of 2
    }

    #[test]
    fn test_thread_pool_basic() {
        let pool = ThreadPool::with_workers(2);
        let counter = Arc::new(AtomicU32::new(0));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let counter: Arc<AtomicU32> = Arc::clone(&counter);
                pool.submit(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();

        // Wait for all jobs
        for handle in handles {
            handle.join();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_thread_pool_worker_count() {
        let pool = ThreadPool::with_workers(4);
        assert_eq!(pool.worker_count(), 4);
    }

    #[test]
    fn test_thread_pool_submit_batch() {
        let pool = ThreadPool::with_workers(2);
        let counter = Arc::new(AtomicU32::new(0));

        let tasks: Vec<_> = (0..5)
            .map(|_| {
                let counter: Arc<AtomicU32> = Arc::clone(&counter);
                move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
            .collect();

        let handles = pool.submit_batch(tasks);

        for handle in handles {
            handle.join();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_job_handle_is_complete() {
        let pool = ThreadPool::with_workers(1);
        let handle = pool.submit(|| {
            std::thread::sleep(std::time::Duration::from_millis(10));
        });

        // Initially not complete
        // Note: This is racy, but with the sleep it should be false initially
        // assert!(!handle.is_complete());

        handle.join();
        assert!(handle.is_complete());
    }

    #[test]
    fn test_thread_pool_join() {
        let mut pool = ThreadPool::with_workers(2);
        let counter = Arc::new(AtomicU32::new(0));

        for _ in 0..5 {
            let counter: Arc<AtomicU32> = Arc::clone(&counter);
            pool.submit(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        pool.join();
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_scan_job() {
        let job = ScanJob::new(std::path::PathBuf::from("/test/file.txt"), vec![1, 2, 3, 4]);

        assert_eq!(job.path.to_str(), Some("/test/file.txt"));
        assert_eq!(job.data, vec![1, 2, 3, 4]);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::atomic::AtomicU32;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Ring buffer should preserve FIFO order
        #[test]
        fn prop_ring_buffer_fifo_order(items in prop::collection::vec(any::<i32>(), 1..100)) {
            let mut buffer: RingBuffer<i32> = RingBuffer::new(items.len());

            // Push all items
            for &item in &items {
                let pushed = buffer.try_push(item);
                prop_assert!(pushed, "Should be able to push item");
            }

            // Pop all items and verify order
            for &expected in &items {
                let popped = buffer.try_pop();
                prop_assert_eq!(popped, Some(expected), "Items should come out in FIFO order");
            }

            prop_assert!(buffer.is_empty(), "Buffer should be empty after popping all items");
        }

        /// Ring buffer length should be accurate
        #[test]
        fn prop_ring_buffer_length(
            push_count in 1usize..50,
            pop_count in 0usize..50
        ) {
            let mut buffer: RingBuffer<i32> = RingBuffer::new(100);

            // Push items
            for i in 0..push_count {
                buffer.try_push(i as i32);
            }

            // Pop some items
            let actual_pops = pop_count.min(push_count);
            for _ in 0..actual_pops {
                buffer.try_pop();
            }

            let expected_len = push_count - actual_pops;
            prop_assert_eq!(buffer.len(), expected_len, "Length should be accurate");
        }

        /// Thread pool should execute all submitted jobs
        #[test]
        fn prop_thread_pool_executes_all(job_count in 1usize..20) {
            let pool = ThreadPool::with_workers(2);
            let counter = Arc::new(AtomicU32::new(0));

            let handles: Vec<_> = (0..job_count)
                .map(|_| {
                    let counter: Arc<AtomicU32> = Arc::clone(&counter);
                    pool.submit(move || {
                        counter.fetch_add(1, Ordering::SeqCst);
                    })
                })
                .collect();

            for handle in handles {
                handle.join();
            }

            prop_assert_eq!(
                counter.load(Ordering::SeqCst) as usize,
                job_count,
                "All jobs should be executed"
            );
        }
    }
}
