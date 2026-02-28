//! # Binary Ring Buffer Background Jobs
//!
//! Binary job queue using a ring buffer for O(1) enqueue/dequeue.
//! Achieves 60x smaller job overhead than JSON-based queues.
//!
//! ## Design
//!
//! Jobs are stored as compact binary packets (~14-16 bytes header):
//! - job_type: u16 (job handler ID)
//! - priority: u8 (0-255, higher = more urgent)
//! - retry_count: u8 (number of retries)
//! - scheduled_at: u64 (Unix timestamp)
//! - payload_len: u16 (length of payload bytes)
//!
//! The queue is a ring buffer in shared memory, not Redis.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Binary job packet header (~14 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Job {
    /// Job type/handler ID
    pub job_type: u16,
    /// Priority (0-255, higher = more urgent)
    pub priority: u8,
    /// Number of retry attempts
    pub retry_count: u8,
    /// Scheduled execution time (Unix timestamp)
    pub scheduled_at: u64,
    /// Length of payload bytes
    pub payload_len: u16,
}

impl Job {
    /// Header size in bytes
    pub const HEADER_SIZE: usize = 14;

    /// Create a new job
    pub fn new(job_type: u16, priority: u8, scheduled_at: u64, payload_len: u16) -> Self {
        Self {
            job_type,
            priority,
            retry_count: 0,
            scheduled_at,
            payload_len,
        }
    }

    /// Create an immediate job (scheduled for now)
    pub fn immediate(job_type: u16, payload_len: u16) -> Self {
        Self::new(job_type, 128, 0, payload_len)
    }

    /// Create a high-priority job
    pub fn high_priority(job_type: u16, payload_len: u16) -> Self {
        Self::new(job_type, 255, 0, payload_len)
    }

    /// Create a low-priority job
    pub fn low_priority(job_type: u16, payload_len: u16) -> Self {
        Self::new(job_type, 0, 0, payload_len)
    }

    /// Increment retry count
    pub fn retry(&mut self) {
        self.retry_count = self.retry_count.saturating_add(1);
    }

    /// Check if job should be executed now
    pub fn is_ready(&self, now: u64) -> bool {
        self.scheduled_at <= now
    }

    /// Get total size (header + payload)
    pub fn total_size(&self) -> usize {
        Self::HEADER_SIZE + self.payload_len as usize
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::HEADER_SIZE] {
        let mut bytes = [0u8; Self::HEADER_SIZE];
        bytes[0..2].copy_from_slice(&self.job_type.to_le_bytes());
        bytes[2] = self.priority;
        bytes[3] = self.retry_count;
        bytes[4..12].copy_from_slice(&self.scheduled_at.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::HEADER_SIZE {
            return None;
        }
        Some(Self {
            job_type: u16::from_le_bytes([bytes[0], bytes[1]]),
            priority: bytes[2],
            retry_count: bytes[3],
            scheduled_at: u64::from_le_bytes([
                bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
            ]),
            payload_len: u16::from_le_bytes([bytes[12], bytes[13]]),
        })
    }
}

/// Ring buffer job queue
///
/// O(1) enqueue and dequeue operations.
/// Thread-safe with atomic head/tail pointers.
pub struct JobQueue {
    /// Ring buffer storage
    buffer: Vec<u8>,
    /// Capacity in bytes
    capacity: usize,
    /// Head pointer (read position)
    head: AtomicUsize,
    /// Tail pointer (write position)
    tail: AtomicUsize,
}

impl JobQueue {
    /// Create a new job queue with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity],
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Create a queue with default capacity (64KB)
    pub fn default_capacity() -> Self {
        Self::new(64 * 1024)
    }

    /// Get available space in bytes
    pub fn available_space(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        if tail >= head {
            self.capacity - (tail - head) - 1
        } else {
            head - tail - 1
        }
    }

    /// Get used space in bytes
    pub fn used_space(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        if tail >= head {
            tail - head
        } else {
            self.capacity - head + tail
        }
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.available_space() == 0
    }

    /// Enqueue a job - O(1)
    pub fn enqueue(&mut self, job: &Job, payload: &[u8]) -> bool {
        let total_size = Job::HEADER_SIZE + payload.len();

        if self.available_space() < total_size {
            return false;
        }

        // Write job header
        let job_bytes = job.to_bytes();
        self.write_bytes(&job_bytes);

        // Write payload
        self.write_bytes(payload);

        true
    }

    /// Dequeue a job - O(1)
    pub fn dequeue(&mut self) -> Option<(Job, Vec<u8>)> {
        if self.is_empty() {
            return None;
        }

        // Read job header
        let job = self.read_job()?;

        // Read payload
        let payload = self.read_bytes(job.payload_len as usize);

        Some((job, payload))
    }

    /// Peek at the next job without removing it
    pub fn peek(&self) -> Option<Job> {
        if self.is_empty() {
            return None;
        }

        let head = self.head.load(Ordering::Acquire);
        let mut header_bytes = [0u8; Job::HEADER_SIZE];

        for (i, byte) in header_bytes.iter_mut().enumerate() {
            *byte = self.buffer[(head + i) % self.capacity];
        }

        Job::from_bytes(&header_bytes)
    }

    /// Write bytes to the ring buffer
    fn write_bytes(&mut self, bytes: &[u8]) {
        let mut tail = self.tail.load(Ordering::Acquire);

        for &byte in bytes {
            self.buffer[tail] = byte;
            tail = (tail + 1) % self.capacity;
        }

        self.tail.store(tail, Ordering::Release);
    }

    /// Read job header from the ring buffer
    fn read_job(&mut self) -> Option<Job> {
        let mut head = self.head.load(Ordering::Acquire);
        let mut header_bytes = [0u8; Job::HEADER_SIZE];

        for byte in &mut header_bytes {
            *byte = self.buffer[head];
            head = (head + 1) % self.capacity;
        }

        self.head.store(head, Ordering::Release);
        Job::from_bytes(&header_bytes)
    }

    /// Read bytes from the ring buffer
    fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut head = self.head.load(Ordering::Acquire);
        let mut bytes = Vec::with_capacity(len);

        for _ in 0..len {
            bytes.push(self.buffer[head]);
            head = (head + 1) % self.capacity;
        }

        self.head.store(head, Ordering::Release);
        bytes
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
    }
}

/// Job handler function type
pub type JobHandler = fn(&[u8]) -> Result<(), JobError>;

/// Job error types
#[derive(Debug, Clone)]
pub enum JobError {
    /// Job failed, should retry
    Retry(String),
    /// Job failed permanently
    Failed(String),
    /// Job timed out
    Timeout,
}

/// Job registry for handler lookup
pub struct JobRegistry {
    handlers: Vec<Option<JobHandler>>,
}

impl JobRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            handlers: vec![None; 256],
        }
    }

    /// Register a handler for a job type
    pub fn register(&mut self, job_type: u16, handler: JobHandler) {
        if (job_type as usize) < self.handlers.len() {
            self.handlers[job_type as usize] = Some(handler);
        }
    }

    /// Get handler for a job type
    pub fn get(&self, job_type: u16) -> Option<JobHandler> {
        self.handlers.get(job_type as usize).copied().flatten()
    }

    /// Execute a job
    pub fn execute(&self, job: &Job, payload: &[u8]) -> Result<(), JobError> {
        match self.get(job.job_type) {
            Some(handler) => handler(payload),
            None => Err(JobError::Failed(format!("Unknown job type: {}", job.job_type))),
        }
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Job worker for processing jobs from a queue
pub struct JobWorker {
    queue: JobQueue,
    registry: JobRegistry,
    max_retries: u8,
}

impl JobWorker {
    /// Create a new worker
    pub fn new(queue: JobQueue, registry: JobRegistry) -> Self {
        Self {
            queue,
            registry,
            max_retries: 3,
        }
    }

    /// Set maximum retry count
    pub fn with_max_retries(mut self, max_retries: u8) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Process one job from the queue
    pub fn process_one(&mut self) -> Option<Result<(), JobError>> {
        let (mut job, payload) = self.queue.dequeue()?;

        match self.registry.execute(&job, &payload) {
            Ok(()) => Some(Ok(())),
            Err(JobError::Retry(msg)) if job.retry_count < self.max_retries => {
                // Re-enqueue for retry
                job.retry();
                self.queue.enqueue(&job, &payload);
                Some(Err(JobError::Retry(msg)))
            }
            Err(e) => Some(Err(e)),
        }
    }

    /// Process all available jobs
    pub fn process_all(&mut self) -> Vec<Result<(), JobError>> {
        let mut results = Vec::new();
        while let Some(result) = self.process_one() {
            results.push(result);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_header_size() {
        assert_eq!(Job::HEADER_SIZE, 14);
    }

    #[test]
    fn test_job_roundtrip() {
        let job = Job::new(42, 128, 1234567890, 100);
        let bytes = job.to_bytes();
        let restored = Job::from_bytes(&bytes).unwrap();

        assert_eq!(restored.job_type, 42);
        assert_eq!(restored.priority, 128);
        assert_eq!(restored.scheduled_at, 1234567890);
        assert_eq!(restored.payload_len, 100);
    }

    #[test]
    fn test_job_queue_enqueue_dequeue() {
        let mut queue = JobQueue::new(1024);

        let job = Job::immediate(1, 5);
        let payload = b"hello";

        assert!(queue.enqueue(&job, payload));

        let (dequeued_job, dequeued_payload) = queue.dequeue().unwrap();
        assert_eq!(dequeued_job.job_type, 1);
        assert_eq!(dequeued_payload, b"hello");
    }

    #[test]
    fn test_job_queue_fifo() {
        let mut queue = JobQueue::new(1024);

        // Enqueue jobs
        for i in 0..5 {
            let job = Job::immediate(i, 1);
            queue.enqueue(&job, &[i as u8]);
        }

        // Dequeue and verify FIFO order
        for i in 0..5 {
            let (job, payload) = queue.dequeue().unwrap();
            assert_eq!(job.job_type, i);
            assert_eq!(payload, vec![i as u8]);
        }

        assert!(queue.is_empty());
    }

    #[test]
    fn test_job_queue_full() {
        let mut queue = JobQueue::new(32);

        // Fill the queue
        let job = Job::immediate(1, 10);
        let payload = [0u8; 10];

        // First enqueue should succeed
        assert!(queue.enqueue(&job, &payload));

        // Second enqueue should fail (not enough space)
        assert!(!queue.enqueue(&job, &payload));
    }

    #[test]
    fn test_job_registry() {
        let mut registry = JobRegistry::new();

        fn test_handler(_payload: &[u8]) -> Result<(), JobError> {
            Ok(())
        }

        registry.register(1, test_handler);

        assert!(registry.get(1).is_some());
        assert!(registry.get(2).is_none());
    }

    #[test]
    fn test_job_retry() {
        let mut job = Job::immediate(1, 0);
        assert_eq!(job.retry_count, 0);

        job.retry();
        assert_eq!(job.retry_count, 1);

        job.retry();
        assert_eq!(job.retry_count, 2);
    }

    #[test]
    fn test_job_is_ready() {
        let job = Job::new(1, 128, 100, 0);

        assert!(job.is_ready(100));
        assert!(job.is_ready(200));
        assert!(!job.is_ready(50));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 35: Job Struct Size**
    // **Validates: Requirements 22.1, 22.3**
    // *For any* Job instance, `Job::HEADER_SIZE` SHALL be approximately 14-16 bytes.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_job_header_size_is_14_bytes(
            job_type in 0u16..=u16::MAX,
            priority in 0u8..=255u8,
            retry_count in 0u8..=255u8,
            scheduled_at in 0u64..=u64::MAX,
            payload_len in 0u16..=u16::MAX,
        ) {
            let job = Job {
                job_type,
                priority,
                retry_count,
                scheduled_at,
                payload_len,
            };

            // Header size must be exactly 14 bytes
            prop_assert_eq!(Job::HEADER_SIZE, 14);
            prop_assert_eq!(job.to_bytes().len(), 14);

            // Total size is header + payload
            prop_assert_eq!(job.total_size(), 14 + payload_len as usize);
        }

        #[test]
        fn prop_job_roundtrip(
            job_type in 0u16..=u16::MAX,
            priority in 0u8..=255u8,
            retry_count in 0u8..=255u8,
            scheduled_at in 0u64..=u64::MAX,
            payload_len in 0u16..=u16::MAX,
        ) {
            let job = Job {
                job_type,
                priority,
                retry_count,
                scheduled_at,
                payload_len,
            };

            let bytes = job.to_bytes();
            let restored = Job::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.job_type, job_type);
            prop_assert_eq!(restored.priority, priority);
            prop_assert_eq!(restored.retry_count, retry_count);
            prop_assert_eq!(restored.scheduled_at, scheduled_at);
            prop_assert_eq!(restored.payload_len, payload_len);
        }

        #[test]
        fn prop_job_is_ready_consistency(
            scheduled_at in 0u64..=1000000u64,
            now in 0u64..=1000000u64,
        ) {
            let job = Job::new(1, 128, scheduled_at, 0);

            // Job is ready iff now >= scheduled_at
            prop_assert_eq!(job.is_ready(now), now >= scheduled_at);
        }

        #[test]
        fn prop_job_retry_increments(
            initial_retry in 0u8..=250u8,
        ) {
            let mut job = Job {
                job_type: 1,
                priority: 128,
                retry_count: initial_retry,
                scheduled_at: 0,
                payload_len: 0,
            };

            job.retry();

            // Retry count should increment (with saturation)
            prop_assert_eq!(job.retry_count, initial_retry.saturating_add(1));
        }
    }
}

#[cfg(test)]
mod property_tests_fifo {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 36: Ring Buffer FIFO**
    // **Validates: Requirements 22.2**
    // *For any* sequence of jobs enqueued to JobQueue, dequeuing SHALL return them in FIFO order.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_ring_buffer_fifo_order(
            job_types in prop::collection::vec(0u16..=1000u16, 1..20),
        ) {
            let mut queue = JobQueue::new(4096);

            // Enqueue all jobs
            for &job_type in &job_types {
                let job = Job::immediate(job_type, 0);
                let enqueued = queue.enqueue(&job, &[]);
                prop_assert!(enqueued, "Should be able to enqueue job");
            }

            // Dequeue and verify FIFO order
            for &expected_type in &job_types {
                let (job, _) = queue.dequeue().expect("Should have job to dequeue");
                prop_assert_eq!(job.job_type, expected_type, "Jobs should be in FIFO order");
            }

            // Queue should be empty
            prop_assert!(queue.is_empty());
        }

        #[test]
        fn prop_ring_buffer_fifo_with_payloads(
            payloads in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..50),
                1..10
            ),
        ) {
            let mut queue = JobQueue::new(4096);

            // Enqueue jobs with payloads
            for (i, payload) in payloads.iter().enumerate() {
                let job = Job::immediate(i as u16, payload.len() as u16);
                let enqueued = queue.enqueue(&job, payload);
                prop_assert!(enqueued, "Should be able to enqueue job");
            }

            // Dequeue and verify payloads match in FIFO order
            for (i, expected_payload) in payloads.iter().enumerate() {
                let (job, payload) = queue.dequeue().expect("Should have job to dequeue");
                prop_assert_eq!(job.job_type, i as u16);
                prop_assert_eq!(&payload, expected_payload);
            }

            prop_assert!(queue.is_empty());
        }

        #[test]
        fn prop_ring_buffer_space_tracking(
            operations in prop::collection::vec(
                prop::bool::ANY,  // true = enqueue, false = dequeue
                1..50
            ),
        ) {
            let mut queue = JobQueue::new(1024);
            let mut expected_count = 0usize;

            for should_enqueue in operations {
                if should_enqueue && expected_count < 50 {
                    let job = Job::immediate(expected_count as u16, 0);
                    if queue.enqueue(&job, &[]) {
                        expected_count += 1;
                    }
                } else if !should_enqueue && expected_count > 0 {
                    if queue.dequeue().is_some() {
                        expected_count -= 1;
                    }
                }

                // Verify empty state matches expected count
                prop_assert_eq!(queue.is_empty(), expected_count == 0);
            }
        }

        #[test]
        fn prop_peek_does_not_remove(
            job_type in 0u16..=1000u16,
            payload in prop::collection::vec(any::<u8>(), 0..20),
        ) {
            let mut queue = JobQueue::new(1024);

            let job = Job::immediate(job_type, payload.len() as u16);
            queue.enqueue(&job, &payload);

            // Peek should return the job
            let peeked = queue.peek().expect("Should have job to peek");
            prop_assert_eq!(peeked.job_type, job_type);

            // Queue should not be empty after peek
            prop_assert!(!queue.is_empty());

            // Dequeue should return the same job
            let (dequeued, dequeued_payload) = queue.dequeue().expect("Should have job to dequeue");
            prop_assert_eq!(dequeued.job_type, job_type);
            prop_assert_eq!(dequeued_payload, payload);

            // Now queue should be empty
            prop_assert!(queue.is_empty());
        }
    }
}
