//! Timing middleware.

use super::{Middleware, MiddlewareResult, Request, Response};
use std::time::Instant;

/// Request timing middleware.
///
/// Measures request duration and adds X-Response-Time header.
pub struct TimingMiddleware;

impl Middleware for TimingMiddleware {
    fn before(req: &mut Request) -> MiddlewareResult<()> {
        req.start_time = Some(Instant::now());
        Ok(())
    }

    fn after(req: &Request, res: &mut Response) {
        if let Some(start_time) = req.start_time {
            let duration = start_time.elapsed();
            let duration_str = format!("{:.3}ms", duration.as_secs_f64() * 1000.0);
            res.set_header("X-Response-Time", duration_str);
        }
    }
}
