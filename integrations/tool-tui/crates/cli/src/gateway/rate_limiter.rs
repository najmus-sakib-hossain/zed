//! Rate limiting for API endpoints

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<IpAddr, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut requests = self.requests.write().await;
        let now = Instant::now();

        let entry = requests.entry(ip).or_insert_with(Vec::new);

        // Remove old requests outside the window
        entry.retain(|&time| now.duration_since(time) < self.window);

        if entry.len() >= self.max_requests {
            return false;
        }

        entry.push(now);
        true
    }

    pub async fn reset(&self, ip: IpAddr) {
        let mut requests = self.requests.write().await;
        requests.remove(&ip);
    }

    pub async fn cleanup(&self) {
        let mut requests = self.requests.write().await;
        let now = Instant::now();

        requests.retain(|_, times| {
            times.retain(|&time| now.duration_since(time) < self.window);
            !times.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(!limiter.check(ip).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(!limiter.check(ip).await);

        limiter.reset(ip).await;
        assert!(limiter.check(ip).await);
    }
}
