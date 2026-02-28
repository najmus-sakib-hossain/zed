use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const LOGICAL_BITS: u32 = 16;
const LOGICAL_MASK: u64 = (1u64 << LOGICAL_BITS) - 1;

/// Hybrid logical clock inspired by HLCs. Encodes physical milliseconds and
/// a logical counter into a single `u64` so ordering stays monotonic even
/// across multiple processes.
pub struct HybridLogicalClock {
    value: AtomicU64,
}

impl Default for HybridLogicalClock {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridLogicalClock {
    pub fn new() -> Self {
        let now = current_millis();
        Self {
            value: AtomicU64::new(encode(now, 0)),
        }
    }

    /// Tick the clock for a local event and return the new hybrid timestamp.
    pub fn tick(&self) -> u64 {
        let physical = current_millis();
        loop {
            let current = self.value.load(Ordering::Relaxed);
            let (cur_phys, cur_logical) = decode(current);

            // If physical time has advanced, reset logical counter
            // Otherwise (physical <= cur_phys), increment logical counter
            let next = if physical > cur_phys {
                encode(physical, 0)
            } else {
                encode(cur_phys, cur_logical.wrapping_add(1))
            };

            if self
                .value
                .compare_exchange(current, next, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return next;
            }
        }
    }

    /// Observe a remote timestamp so subsequent local ticks stay ahead.
    pub fn observe(&self, external: u64) {
        loop {
            let current = self.value.load(Ordering::Relaxed);
            if current >= external {
                return;
            }
            if self
                .value
                .compare_exchange(current, external, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }
    }
}

fn current_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

fn encode(physical: u64, logical: u64) -> u64 {
    (physical << LOGICAL_BITS) | (logical & LOGICAL_MASK)
}

fn decode(value: u64) -> (u64, u64) {
    (value >> LOGICAL_BITS, value & LOGICAL_MASK)
}

pub static GLOBAL_CLOCK: Lazy<HybridLogicalClock> = Lazy::new(HybridLogicalClock::new);
