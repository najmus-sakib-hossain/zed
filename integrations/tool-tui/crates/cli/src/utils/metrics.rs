//! Metrics collection for monitoring

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub struct Metrics {
    messages_sent: Arc<AtomicU64>,
    messages_received: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    active_connections: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            messages_sent: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn increment_messages_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_messages_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get_messages_sent(&self) -> u64 {
        self.messages_sent.load(Ordering::Relaxed)
    }

    pub fn get_messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    pub fn get_errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    pub fn get_active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.messages_sent.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
