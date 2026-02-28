//! Rate limiter for API providers — enforces RPM/TPM limits per API key.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A sliding-window rate limiter that tracks requests per minute.
pub struct RateLimiter {
    inner: Mutex<RateLimiterInner>,
}

struct RateLimiterInner {
    /// Maximum requests per minute.
    rpm_limit: u32,
    /// Timestamps of recent requests.
    window: VecDeque<Instant>,
    /// Window duration (60 seconds).
    window_duration: Duration,
}

impl RateLimiter {
    /// Create a rate limiter with the given RPM limit.
    pub fn new(rpm_limit: u32) -> Self {
        Self {
            inner: Mutex::new(RateLimiterInner {
                rpm_limit,
                window: VecDeque::new(),
                window_duration: Duration::from_secs(60),
            }),
        }
    }

    /// Check if a request can proceed. Returns true if allowed.
    pub fn try_acquire(&self) -> bool {
        let mut inner = self.inner.lock();
        let now = Instant::now();
        let cutoff = now - inner.window_duration;

        // Remove expired entries.
        while inner.window.front().is_some_and(|t| *t < cutoff) {
            inner.window.pop_front();
        }

        if (inner.window.len() as u32) < inner.rpm_limit {
            inner.window.push_back(now);
            true
        } else {
            false
        }
    }

    /// How long until the next request can be made (0 if immediately available).
    pub fn wait_duration(&self) -> Duration {
        let inner = self.inner.lock();
        let now = Instant::now();
        let cutoff = now - inner.window_duration;

        if (inner.window.len() as u32) < inner.rpm_limit {
            return Duration::ZERO;
        }

        // The oldest request in the window determines when the next slot opens.
        if let Some(oldest) = inner.window.front() {
            if *oldest >= cutoff {
                return (*oldest + inner.window_duration) - now;
            }
        }
        Duration::ZERO
    }

    /// Current number of requests in the window.
    pub fn current_count(&self) -> u32 {
        let mut inner = self.inner.lock();
        let now = Instant::now();
        let cutoff = now - inner.window_duration;

        while inner.window.front().is_some_and(|t| *t < cutoff) {
            inner.window.pop_front();
        }

        inner.window.len() as u32
    }

    /// The configured RPM limit.
    pub fn rpm_limit(&self) -> u32 {
        self.inner.lock().rpm_limit
    }
}
