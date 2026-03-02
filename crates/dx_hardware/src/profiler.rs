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

    /// Total RAM in gigabytes.
    pub fn ram_gb(&self) -> f64 {
        self.ram_bytes as f64 / 1_073_741_824.0
    }

    /// GPU VRAM in gigabytes.
    pub fn vram_gb(&self) -> f64 {
        self.gpu.vram_bytes as f64 / 1_073_741_824.0
    }

    /// Number of CPU cores.
    pub fn cpu_cores(&self) -> usize {
        self.cpu_cores
    }

    /// Whether the system has an SSD (heuristic based on disk speed).
    pub fn has_ssd(&self) -> bool {
        // Conservative heuristic: if disk is >100GB total, likely SSD
        // Real implementation would check device type via platform API
        self.disk_total_bytes > 100 * 1_073_741_824
    }

    /// Available disk space in gigabytes.
    pub fn disk_free_gb(&self) -> f64 {
        self.disk_available_bytes as f64 / 1_073_741_824.0
    }
}

fn classify_tier(cpu_cores: usize, ram_bytes: u64, gpu: &GpuCapability) -> DeviceTier {
    let ram_gb = ram_bytes / 1_073_741_824;
    let vram_gb = gpu.vram_bytes / 1_073_741_824;

    if vram_gb >= 24 && ram_gb >= 32 && cpu_cores >= 8 {
        DeviceTier::Ultra
    } else if vram_gb >= 8 && ram_gb >= 16 && cpu_cores >= 6 {
        DeviceTier::High
    } else if ram_gb >= 8 && cpu_cores >= 4 {
        DeviceTier::Mid
    } else if ram_gb >= 4 && cpu_cores >= 2 {
        DeviceTier::Low
    } else {
        DeviceTier::UltraLow
    }
}

fn detect_cpu_model() -> String {
    #[cfg(target_os = "windows")]
    {
        // Try Windows environment variable first
        if let Ok(id) = std::env::var("PROCESSOR_IDENTIFIER") {
            return id;
        }
        // Fallback: try WMIC
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["cpu", "get", "Name", "/format:list"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(name) = line.strip_prefix("Name=") {
                    return name.trim().to_string();
                }
            }
        }
        "Unknown CPU".into()
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
        "Apple Silicon".into()
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in contents.lines() {
                if line.starts_with("model name") {
                    if let Some(name) = line.split(':').nth(1) {
                        return name.trim().to_string();
                    }
                }
            }
        }
        "Unknown CPU".into()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "Unknown CPU".into()
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map_or(2, |n| n.get())
}

fn total_ram() -> u64 {
    #[cfg(target_os = "windows")]
    {
        // Use Windows GlobalMemoryStatusEx via wmic
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["OS", "get", "TotalVisibleMemorySize", "/format:list"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(val) = line.strip_prefix("TotalVisibleMemorySize=") {
                    if let Ok(kb) = val.trim().parse::<u64>() {
                        return kb * 1024; // KB to bytes
                    }
                }
            }
        }
        8 * 1_073_741_824 // fallback: 8 GB
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Ok(bytes) = text.trim().parse::<u64>() {
                    return bytes;
                }
            }
        }
        8 * 1_073_741_824
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return kb * 1024;
                        }
                    }
                }
            }
        }
        8 * 1_073_741_824
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        8 * 1_073_741_824
    }
}

fn available_ram() -> u64 {
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["OS", "get", "FreePhysicalMemory", "/format:list"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(val) = line.strip_prefix("FreePhysicalMemory=") {
                    if let Ok(kb) = val.trim().parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
        4 * 1_073_741_824
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemAvailable:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return kb * 1024;
                        }
                    }
                }
            }
        }
        4 * 1_073_741_824
    }
    #[cfg(target_os = "macos")]
    {
        // macOS: use vm_stat to estimate available memory
        if let Ok(output) = std::process::Command::new("vm_stat").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut free_pages = 0u64;
            let mut inactive_pages = 0u64;
            let page_size = 16384u64; // ARM64 macOS uses 16KB pages
            for line in text.lines() {
                if line.starts_with("Pages free:") {
                    if let Some(val) = line.split(':').nth(1) {
                        free_pages = val.trim().trim_end_matches('.').parse().unwrap_or(0);
                    }
                } else if line.starts_with("Pages inactive:") {
                    if let Some(val) = line.split(':').nth(1) {
                        inactive_pages = val.trim().trim_end_matches('.').parse().unwrap_or(0);
                    }
                }
            }
            return (free_pages + inactive_pages) * page_size;
        }
        4 * 1_073_741_824
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        4 * 1_073_741_824
    }
}

fn disk_space() -> (u64, u64) {
    #[cfg(target_os = "windows")]
    {
        // Use wmic to get disk space on C: drive
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["logicaldisk", "where", "DeviceID='C:'", "get", "Size,FreeSpace", "/format:list"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut total = 0u64;
            let mut free = 0u64;
            for line in text.lines() {
                if let Some(val) = line.strip_prefix("Size=") {
                    total = val.trim().parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("FreeSpace=") {
                    free = val.trim().parse().unwrap_or(0);
                }
            }
            if total > 0 {
                return (total, free);
            }
        }
        (500 * 1_073_741_824, 50 * 1_073_741_824)
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = std::process::Command::new("df")
            .args(["-B1", "/"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let total = parts[1].parse().unwrap_or(0);
                    let avail = parts[3].parse().unwrap_or(0);
                    return (total, avail);
                }
            }
        }
        (500 * 1_073_741_824, 50 * 1_073_741_824)
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("df")
            .args(["-b", "/"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let total: u64 = parts[1].parse().unwrap_or(0) * 512; // 512-byte blocks
                    let avail: u64 = parts[3].parse().unwrap_or(0) * 512;
                    return (total, avail);
                }
            }
        }
        (500 * 1_073_741_824, 50 * 1_073_741_824)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        (500 * 1_073_741_824, 50 * 1_073_741_824)
    }
}
