//! Hardware profiler — detects system capabilities.

use dx_core::DeviceTier;
use serde::{Deserialize, Serialize};

use crate::gpu::GpuCapability;

/// Full hardware profile of the current machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// CPU model name.
    pub cpu_model: String,
    /// Number of logical CPU cores.
    pub cpu_cores: usize,
    /// Total RAM in bytes.
    pub ram_bytes: u64,
    /// Available RAM in bytes.
    pub ram_available_bytes: u64,
    /// Total disk space in bytes (on the application's partition).
    pub disk_total_bytes: u64,
    /// Available disk space in bytes.
    pub disk_available_bytes: u64,
    /// GPU capabilities.
    pub gpu: GpuCapability,
    /// Detected device tier.
    pub tier: DeviceTier,
    /// Operating system.
    pub os: String,
    /// Architecture (x86_64, aarch64, etc.).
    pub arch: String,
}

impl HardwareProfile {
    /// Detect current hardware. Uses platform APIs when available,
    /// falls back to conservative estimates.
    pub fn detect() -> Self {
        let cpu_model = detect_cpu_model();
        let cpu_cores = num_cpus();
        let ram_bytes = total_ram();
        let ram_available_bytes = available_ram();
        let (disk_total, disk_available) = disk_space();
        let gpu = GpuCapability::detect();
        let tier = classify_tier(cpu_cores, ram_bytes, &gpu);
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();

        Self {
            cpu_model,
            cpu_cores,
            ram_bytes,
            ram_available_bytes,
            disk_total_bytes: disk_total,
            disk_available_bytes: disk_available,
            gpu,
            tier,
            os,
            arch,
        }
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "{} cores, {:.1} GB RAM ({:.1} GB free), GPU: {}, Tier: {:?}",
            self.cpu_cores,
            self.ram_bytes as f64 / 1_073_741_824.0,
            self.ram_available_bytes as f64 / 1_073_741_824.0,
            self.gpu.summary(),
            self.tier,
        )
    }
}

fn classify_tier(cpu_cores: usize, ram_bytes: u64, gpu: &GpuCapability) -> DeviceTier {
    let ram_gb = ram_bytes / 1_073_741_824;
    let vram_gb = gpu.vram_bytes / 1_073_741_824;

    if vram_gb >= 24 && ram_gb >= 32 && cpu_cores >= 8 {
        DeviceTier::Workstation
    } else if vram_gb >= 8 && ram_gb >= 16 && cpu_cores >= 6 {
        DeviceTier::HighEnd
    } else if ram_gb >= 8 && cpu_cores >= 4 {
        DeviceTier::MidRange
    } else if ram_gb >= 4 && cpu_cores >= 2 {
        DeviceTier::LowEnd
    } else {
        DeviceTier::Embedded
    }
}

fn detect_cpu_model() -> String {
    // Simplified — real implementation would use sysinfo crate
    #[cfg(target_os = "windows")]
    {
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Unknown CPU".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        "Unknown CPU".into()
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map_or(2, |n| n.get())
}

fn total_ram() -> u64 {
    // Placeholder — real implementation would use sysinfo or platform APIs
    #[cfg(target_os = "windows")]
    {
        // Try to read from Windows API if available
        8 * 1_073_741_824 // Conservative default: 8 GB
    }
    #[cfg(not(target_os = "windows"))]
    {
        8 * 1_073_741_824
    }
}

fn available_ram() -> u64 {
    // Placeholder
    4 * 1_073_741_824
}

fn disk_space() -> (u64, u64) {
    // Placeholder
    (500 * 1_073_741_824, 50 * 1_073_741_824)
}
