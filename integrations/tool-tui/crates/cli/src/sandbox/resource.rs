//! Resource monitoring and enforcement

use anyhow::Result;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub disk_bytes: u64,
    pub process_count: usize,
}

/// Resource monitor for tracking sandbox resource usage
pub struct ResourceMonitor {
    system: System,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
            ),
        }
    }

    /// Get current resource usage
    pub fn get_usage(&mut self) -> ResourceUsage {
        self.system.refresh_all();

        let memory_bytes = self.system.used_memory();
        let cpu_percent = self.system.global_cpu_usage();
        let process_count = self.system.processes().len();

        ResourceUsage {
            memory_bytes,
            cpu_percent,
            disk_bytes: 0, // TODO: Implement disk usage tracking
            process_count,
        }
    }

    /// Check if resource limits are exceeded
    pub fn check_limits(&mut self, limits: &super::config::ResourceLimits) -> Result<()> {
        let usage = self.get_usage();

        if let Some(max_memory) = limits.memory_bytes {
            if usage.memory_bytes > max_memory {
                return Err(anyhow::anyhow!(
                    "Memory limit exceeded: {} > {}",
                    usage.memory_bytes,
                    max_memory
                ));
            }
        }

        if let Some(max_pids) = limits.max_pids {
            if usage.process_count > max_pids as usize {
                return Err(anyhow::anyhow!(
                    "Process limit exceeded: {} > {}",
                    usage.process_count,
                    max_pids
                ));
            }
        }

        Ok(())
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
