//! Memory Profiler with allocation tracking

use std::collections::HashMap;
use std::time::Instant;

pub struct MemoryProfiler {
    allocations: Vec<Allocation>,
    current_usage: usize,
    peak_usage: usize,
    active: bool,
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub id: u64,
    pub size: usize,
    pub stack: Vec<String>,
    pub allocated_at: Instant,
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self {
            allocations: Vec::new(),
            current_usage: 0,
            peak_usage: 0,
            active: false,
        }
    }

    pub fn start(&mut self) {
        self.active = true;
        self.allocations.clear();
        self.current_usage = 0;
        self.peak_usage = 0;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn track_allocation(&mut self, id: u64, size: usize, stack: Vec<String>) {
        if !self.active {
            return;
        }
        self.allocations.push(Allocation {
            id,
            size,
            stack,
            allocated_at: Instant::now(),
        });
        self.current_usage += size;
        if self.current_usage > self.peak_usage {
            self.peak_usage = self.current_usage;
        }
    }

    pub fn track_deallocation(&mut self, size: usize) {
        if !self.active {
            return;
        }
        self.current_usage = self.current_usage.saturating_sub(size);
    }

    pub fn get_snapshot(&self) -> MemorySnapshot {
        let mut by_location: HashMap<String, usize> = HashMap::new();
        for alloc in &self.allocations {
            if let Some(loc) = alloc.stack.first() {
                *by_location.entry(loc.clone()).or_insert(0) += alloc.size;
            }
        }

        MemorySnapshot {
            current_usage: self.current_usage,
            peak_usage: self.peak_usage,
            allocation_count: self.allocations.len(),
            by_location,
        }
    }
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemorySnapshot {
    pub current_usage: usize,
    pub peak_usage: usize,
    pub allocation_count: usize,
    pub by_location: HashMap<String, usize>,
}

impl MemorySnapshot {
    pub fn top_allocators(&self, limit: usize) -> Vec<(String, usize)> {
        let mut items: Vec<_> = self.by_location.iter().map(|(k, &v)| (k.clone(), v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(limit);
        items
    }
}
