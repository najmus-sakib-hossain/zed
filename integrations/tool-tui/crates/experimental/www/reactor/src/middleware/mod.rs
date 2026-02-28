//! Compiler-Inlined Middleware (CIM).
//!
//! This module provides middleware that is inlined at compile time for zero runtime overhead.

mod auth;
mod rate_limit;
mod timing;
mod traits;

pub use auth::AuthMiddleware;
pub use rate_limit::{RateLimitMiddleware, get_thread_rate_count, reset_thread_rate_limit};
pub use timing::TimingMiddleware;
pub use traits::{Middleware, MiddlewareError, MiddlewareResult, Request, Response};

/// Macro for compile-time middleware chaining.
///
/// This macro generates a single function with all middleware logic inlined,
/// executing `before()` hooks in order and `after()` hooks in reverse order.
///
/// # Example
///
/// ```ignore
/// dx_middleware!(AuthMiddleware, TimingMiddleware, RateLimitMiddleware);
/// ```
#[macro_export]
macro_rules! dx_middleware {
    ($($middleware:ty),* $(,)?) => {
        pub fn process_middleware(
            req: &mut $crate::middleware::Request,
            res: &mut $crate::middleware::Response,
            handler: impl FnOnce(&$crate::middleware::Request) -> $crate::middleware::MiddlewareResult<()>,
        ) -> $crate::middleware::MiddlewareResult<()> {
            // Execute before hooks in order
            $(
                <$middleware as $crate::middleware::Middleware>::before(req)?;
            )*

            // Execute the main handler
            handler(req)?;

            // Execute after hooks in reverse order
            $crate::dx_middleware!(@reverse_after req, res, $($middleware),*);

            Ok(())
        }
    };

    // Helper to reverse the after hooks
    (@reverse_after $req:expr, $res:expr, $first:ty $(, $rest:ty)*) => {
        $crate::dx_middleware!(@reverse_after $req, $res, $($rest),*);
        <$first as $crate::middleware::Middleware>::after($req, $res);
    };

    (@reverse_after $req:expr, $res:expr,) => {};
}
