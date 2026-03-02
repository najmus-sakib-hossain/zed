//! Rate limiting middleware using Governor.

// TODO: Implement tower-compatible rate limiting layer
// using the `governor` crate.
//
// Example approach:
// - Use a keyed rate limiter (key = client IP)
// - Configure from Settings.rate_limit
// - Return 429 Too Many Requests when exceeded
