//! Profiler module for performance analysis

pub mod cpu;
pub mod flamegraph;
pub mod memory;

pub use cpu::{CpuProfile, CpuProfiler};
pub use flamegraph::FlameGraph;
pub use memory::{MemoryProfiler, MemorySnapshot};

pub struct Profiler {
    pub cpu: CpuProfiler,
    pub memory: MemoryProfiler,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            cpu: CpuProfiler::new(1),
            memory: MemoryProfiler::new(),
        }
    }

    pub fn start_all(&mut self) {
        self.cpu.start();
        self.memory.start();
    }

    pub fn stop_all(&mut self) {
        self.cpu.stop();
        self.memory.stop();
    }

    pub fn generate_report(&self) -> ProfileReport {
        ProfileReport {
            cpu_profile: self.cpu.get_profile(),
            memory_snapshot: self.memory.get_snapshot(),
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ProfileReport {
    pub cpu_profile: CpuProfile,
    pub memory_snapshot: MemorySnapshot,
}
