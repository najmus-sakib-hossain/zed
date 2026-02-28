//! Example of using Compiler-Inlined Middleware (CIM).
//!
//! This example demonstrates how to create a middleware chain
//! that is inlined at compile time for zero runtime overhead.

use dx_reactor::dx_middleware;
use dx_reactor::middleware::{
    RateLimitMiddleware, Request, Response, TimingMiddleware, reset_thread_rate_limit,
};

// Create a middleware chain using the dx_middleware! macro
// This generates a single function with all middleware logic inlined
dx_middleware!(TimingMiddleware, RateLimitMiddleware);

fn main() {
    println!("Compiler-Inlined Middleware Example");
    println!("====================================\n");

    // Reset rate limit for this example
    reset_thread_rate_limit();

    // Create a request
    let mut req = Request::new("/api/users".to_string(), "GET".to_string());
    let mut res = Response::new();

    // Process through the middleware chain
    let result = process_middleware(&mut req, &mut res, |_req| {
        // This is the main handler
        println!("Handler executed!");
        Ok(())
    });

    match result {
        Ok(()) => {
            println!("\nRequest processed successfully!");

            // Check response headers
            if let Some(timing) = res.header("X-Response-Time") {
                println!("Response time: {}", timing);
            }
            if let Some(limit) = res.header("X-RateLimit-Limit") {
                println!("Rate limit: {}", limit);
            }
            if let Some(remaining) = res.header("X-RateLimit-Remaining") {
                println!("Remaining: {}", remaining);
            }
        }
        Err(e) => {
            println!("Request failed: {}", e);
        }
    }
}
