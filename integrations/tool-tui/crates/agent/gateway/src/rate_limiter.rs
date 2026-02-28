//! Rate limiter for the gateway using sliding window algorithm.

use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Sliding window rate limiter
pub struct RateLimiter {
    /// Map of IP -> list of request timestamps
    requests: Arc<DashMap<IpAddr, Vec<Instant>>>,
    /// Maximum requests per window
    max_requests: usize,
    /// Time window duration
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(DashMap::new()),
            max_requests,
            window,
        }
    }

    /// Check if request should be rate limited. Returns true if BLOCKED.
    pub async fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut entry = self.requests.entry(ip).or_insert_with(Vec::new);

        // Remove expired entries
        entry.retain(|&t| now.duration_since(t) < self.window);

        if entry.len() >= self.max_requests {
            true // Rate limited
        } else {
            entry.push(now);
            false // Allowed
        }
    }

    /// Reset rate limit for an IP
    pub async fn reset(&self, ip: IpAddr) {
        self.requests.remove(&ip);
    }

    /// Clean up expired entries for all IPs
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let window = self.window;

        // Collect IPs to clean up
        let ips: Vec<IpAddr> = self.requests.iter().map(|r| *r.key()).collect();

        for ip in ips {
            if let Some(mut entry) = self.requests.get_mut(&ip) {
                entry.retain(|&t| now.duration_since(t) < window);
                if entry.is_empty() {
                    drop(entry);
                    self.requests.remove(&ip);
                }
            }
        }
    }

    /// Get current request count for an IP
    pub fn count(&self, ip: IpAddr) -> usize {
        self.requests
            .get(&ip)
            .map(|entry| {
                let now = Instant::now();
                entry.iter().filter(|&&t| now.duration_since(t) < self.window).count()
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_rate_limiter_allows() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 5 requests should pass
        for _ in 0..5 {
            assert!(!limiter.check(ip).await, "should be allowed");
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Fill up the limit
        for _ in 0..3 {
            assert!(!limiter.check(ip).await);
        }

        // 4th request should be blocked
        assert!(limiter.check(ip).await, "should be blocked");
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        assert!(!limiter.check(ip).await);
        assert!(!limiter.check(ip).await);
        assert!(limiter.check(ip).await); // blocked

        limiter.reset(ip).await;
        assert!(!limiter.check(ip).await); // allowed again
    }

    #[tokio::test]
    async fn test_rate_limiter_per_ip() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        assert!(!limiter.check(ip1).await);
        assert!(limiter.check(ip1).await); // ip1 blocked

        assert!(!limiter.check(ip2).await); // ip2 still allowed
    }
}
