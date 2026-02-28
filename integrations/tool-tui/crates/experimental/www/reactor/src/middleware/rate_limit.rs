//! Rate limiting middleware.

use super::{Middleware, MiddlewareError, MiddlewareResult, Request, Response};
use std::cell::RefCell;
use std::time::{Duration, Instant};

/// Thread-local rate limit state.
struct RateLimitState {
    /// Request count in current window.
    count: u32,
    /// Window start time.
    window_start: Instant,
}

thread_local! {
    /// Thread-local rate limit counter.
    /// Each thread has its own counter to avoid lock contention.
    static RATE_LIMIT: RefCell<RateLimitState> = RefCell::new(RateLimitState {
        count: 0,
        window_start: Instant::now(),
    });
}

/// Rate limiting middleware with thread-local counters.
///
/// Uses thread-local storage to avoid lock contention in the hot path.
pub struct RateLimitMiddleware;

impl RateLimitMiddleware {
    /// Maximum requests per window.
    const MAX_REQUESTS: u32 = 1000;

    /// Window duration.
    const WINDOW_DURATION: Duration = Duration::from_secs(1);
}

impl Middleware for RateLimitMiddleware {
    fn before(_req: &mut Request) -> MiddlewareResult<()> {
        RATE_LIMIT.with(|state| {
            let mut state = state.borrow_mut();
            let now = Instant::now();

            // Check if we need to reset the window
            if now.duration_since(state.window_start) >= Self::WINDOW_DURATION {
                state.count = 0;
                state.window_start = now;
            }

            // Check rate limit
            if state.count >= Self::MAX_REQUESTS {
                return Err(MiddlewareError::RateLimited(format!(
                    "exceeded {} requests per second",
                    Self::MAX_REQUESTS
                )));
            }

            // Increment counter
            state.count += 1;

            Ok(())
        })
    }

    fn after(_req: &Request, res: &mut Response) {
        // Add rate limit headers
        RATE_LIMIT.with(|state| {
            let state = state.borrow();
            let remaining = Self::MAX_REQUESTS.saturating_sub(state.count);
            res.set_header("X-RateLimit-Limit", Self::MAX_REQUESTS.to_string());
            res.set_header("X-RateLimit-Remaining", remaining.to_string());
        });
    }
}

/// Get the current thread's rate limit count (for testing).
pub fn get_thread_rate_count() -> u32 {
    RATE_LIMIT.with(|state| state.borrow().count)
}

/// Reset the current thread's rate limit state (for testing).
pub fn reset_thread_rate_limit() {
    RATE_LIMIT.with(|state| {
        let mut state = state.borrow_mut();
        state.count = 0;
        state.window_start = Instant::now();
    });
}
