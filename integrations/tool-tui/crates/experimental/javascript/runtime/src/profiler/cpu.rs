//! CPU Profiler with sampling and stack traces

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CpuProfiler {
    samples: Vec<Sample>,
    start_time: Instant,
    sample_interval: Duration,
    active: bool,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub timestamp: Duration,
    pub stack: Vec<String>,
    pub cpu_time: u64,
}

impl CpuProfiler {
    pub fn new(sample_interval_ms: u64) -> Self {
        Self {
            samples: Vec::new(),
            start_time: Instant::now(),
            sample_interval: Duration::from_millis(sample_interval_ms),
            active: false,
        }
    }

    pub fn start(&mut self) {
        self.active = true;
        self.start_time = Instant::now();
        self.samples.clear();
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn sample(&mut self, stack: Vec<String>) {
        if !self.active {
            return;
        }
        self.samples.push(Sample {
            timestamp: self.start_time.elapsed(),
            stack,
            cpu_time: 0,
        });
    }

    pub fn get_profile(&self) -> CpuProfile {
        let mut call_counts: HashMap<String, u64> = HashMap::new();
        let mut total_time: HashMap<String, Duration> = HashMap::new();

        for sample in &self.samples {
            for func in &sample.stack {
                *call_counts.entry(func.clone()).or_insert(0) += 1;
                *total_time.entry(func.clone()).or_insert(Duration::ZERO) += self.sample_interval;
            }
        }

        CpuProfile {
            total_samples: self.samples.len(),
            call_counts,
            total_time,
            duration: self.start_time.elapsed(),
        }
    }
}

pub struct CpuProfile {
    pub total_samples: usize,
    pub call_counts: HashMap<String, u64>,
    pub total_time: HashMap<String, Duration>,
    pub duration: Duration,
}

impl CpuProfile {
    pub fn hot_functions(&self, limit: usize) -> Vec<(String, u64, Duration)> {
        let mut funcs: Vec<_> = self
            .call_counts
            .iter()
            .map(|(name, &count)| {
                let time = self.total_time.get(name).copied().unwrap_or(Duration::ZERO);
                (name.clone(), count, time)
            })
            .collect();
        funcs.sort_by(|a, b| b.2.cmp(&a.2));
        funcs.truncate(limit);
        funcs
    }
}
