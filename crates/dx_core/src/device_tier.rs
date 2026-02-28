//! Hardware device tier classification for adaptive model selection.
//!
//! DX profiles hardware at first launch and classifies the device into one of
//! five tiers, determining which local models to download and run.
//!
//! `HardwareProfile::detect()` performs real hardware profiling using the `sysinfo`
//! crate and platform-specific commands (nvidia-smi, rocm-smi, system_profiler, etc.)
//! to determine RAM, VRAM, CPU cores, GPU capabilities, disk space, and battery state.

use serde::{Deserialize, Serialize};
use std::path::Path;
use sysinfo::{Components, Disks, MemoryRefreshKind, RefreshKind, System};

/// The five hardware tiers that determine local model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DeviceTier {
    /// Tier 1: Ultra-Low-End (2–4GB RAM, No GPU)
    /// Raspberry Pi 4, Chromebooks, 10-year-old laptops.
    /// ~615MB total model footprint.
    UltraLow,

    /// Tier 2: Low-End (4–8GB RAM, No GPU)
    /// Entry-level laptops, older MacBooks, budget desktops.
    /// ~1.0GB total model footprint.
    Low,

    /// Tier 3: Mid-Range (8–16GB RAM, iGPU or entry GPU)
    /// MacBook Air M1/M2, mid-range gaming PCs.
    /// ~4.8GB total model footprint.
    Mid,

    /// Tier 4: High-End (16–32GB RAM, discrete GPU 6–12GB VRAM)
    /// MacBook Pro M3 Pro/Max, RTX 3070/4070.
    /// ~20.5GB total model footprint.
    High,

    /// Tier 5: Ultra-High-End (32GB+ RAM, 16GB+ VRAM or Apple Silicon 64GB+)
    /// Mac Studio M3 Ultra, RTX 4090, multi-GPU workstations.
    /// ~100GB total model footprint.
    Ultra,
}

impl DeviceTier {
    /// Classify a device based on available RAM and VRAM.
    pub fn classify(ram_gb: f64, vram_gb: Option<f64>) -> Self {
        let vram = vram_gb.unwrap_or(0.0);

        if ram_gb >= 32.0 && vram >= 16.0 {
            DeviceTier::Ultra
        } else if ram_gb >= 16.0 && vram >= 6.0 {
            DeviceTier::High
        } else if ram_gb >= 8.0 {
            DeviceTier::Mid
        } else if ram_gb >= 4.0 {
            DeviceTier::Low
        } else {
            DeviceTier::UltraLow
        }
    }

    /// Human-readable name for the tier.
    pub fn display_name(&self) -> &'static str {
        match self {
            DeviceTier::UltraLow => "Ultra-Low-End (Tier 1)",
            DeviceTier::Low => "Low-End (Tier 2)",
            DeviceTier::Mid => "Mid-Range (Tier 3)",
            DeviceTier::High => "High-End (Tier 4)",
            DeviceTier::Ultra => "Ultra-High-End (Tier 5)",
        }
    }

    /// Total model footprint for this tier in GB.
    pub fn model_footprint_gb(&self) -> f64 {
        match self {
            DeviceTier::UltraLow => 0.4,
            DeviceTier::Low => 0.75,
            DeviceTier::Mid => 4.0,
            DeviceTier::High => 16.4,
            DeviceTier::Ultra => 90.0,
        }
    }

    /// Whether this tier supports local image generation.
    pub fn supports_local_image_gen(&self) -> bool {
        matches!(self, DeviceTier::High | DeviceTier::Ultra)
    }

    /// Whether this tier supports local 3D generation.
    pub fn supports_local_3d_gen(&self) -> bool {
        matches!(self, DeviceTier::Ultra)
    }

    /// Whether this tier supports Chatterbox-quality TTS.
    pub fn supports_chatterbox_tts(&self) -> bool {
        matches!(self, DeviceTier::High | DeviceTier::Ultra)
    }

    /// Whether this tier supports voice cloning.
    pub fn supports_voice_cloning(&self) -> bool {
        matches!(self, DeviceTier::Ultra)
    }
}

/// Snapshot of detected hardware capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// Total system RAM in GB.
    pub ram_gb: f64,
    /// GPU VRAM in GB (if discrete GPU present).
    pub vram_gb: Option<f64>,
    /// CPU core count.
    pub cpu_cores: usize,
    /// GPU name (e.g., "NVIDIA RTX 4070").
    pub gpu_name: Option<String>,
    /// Whether CUDA is available.
    pub has_cuda: bool,
    /// Whether Metal is available (macOS).
    pub has_metal: bool,
    /// Whether ROCm is available (AMD).
    pub has_rocm: bool,
    /// Whether DirectML is available (Windows).
    pub has_directml: bool,
    /// Whether an NPU/TPU is available.
    pub has_npu: bool,
    /// Classified device tier.
    pub tier: DeviceTier,
    /// Available disk space in GB.
    pub disk_available_gb: f64,
    /// Whether the device is on battery power.
    pub on_battery: bool,
}

impl HardwareProfile {
    /// Create a profile from basic system info and classify the tier.
    pub fn new(ram_gb: f64, vram_gb: Option<f64>, cpu_cores: usize) -> Self {
        let tier = DeviceTier::classify(ram_gb, vram_gb);
        Self {
            ram_gb,
            vram_gb,
            cpu_cores,
            gpu_name: None,
            has_cuda: false,
            has_metal: cfg!(target_os = "macos"),
            has_rocm: false,
            has_directml: cfg!(target_os = "windows"),
            has_npu: false,
            tier,
            disk_available_gb: 0.0,
            on_battery: false,
        }
    }

    /// Detect hardware capabilities from the current system.
    ///
    /// This is the real hardware profiling entry point. It queries:
    /// - Total system RAM via `sysinfo`
    /// - CPU core count via `sysinfo`
    /// - GPU name and VRAM via platform-specific detection
    /// - CUDA availability (NVIDIA `nvidia-smi`)
    /// - ROCm availability (AMD `rocm-smi`)
    /// - Metal availability (macOS compile-time flag)
    /// - DirectML availability (Windows compile-time flag)
    /// - Available disk space via `sysinfo::Disks`
    /// - Battery / AC power state via platform-specific checks
    ///
    /// Call this at first launch to classify the device tier and determine
    /// which local models to download.
    pub fn detect() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_memory(MemoryRefreshKind::everything())
                .with_cpu(sysinfo::CpuRefreshKind::everything()),
        );

        let ram_gb = system.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
        let cpu_cores = system.cpus().len();

        // --- GPU detection ---
        let gpu_info = detect_gpu();
        let vram_gb = gpu_info.vram_gb;
        let gpu_name = gpu_info.name;
        let has_cuda = gpu_info.has_cuda;
        let has_rocm = gpu_info.has_rocm;

        // --- Disk space for model storage ---
        let disk_available_gb = detect_disk_space_gb();

        // --- Battery state ---
        let on_battery = detect_battery_state();

        let has_metal = cfg!(target_os = "macos");
        let has_directml = cfg!(target_os = "windows");

        // On Apple Silicon, unified memory means GPU can use most of system RAM.
        // Treat Apple Silicon with >=16GB as having ~75% of RAM available as "VRAM".
        let effective_vram = if has_metal && vram_gb.is_none() && ram_gb >= 16.0 {
            Some(ram_gb * 0.75)
        } else {
            vram_gb
        };

        let tier = DeviceTier::classify(ram_gb, effective_vram);

        log::info!(
            "DX Hardware Profile: RAM={:.1}GB, VRAM={}, CPU={} cores, GPU={}, Tier={}, Disk={:.1}GB free, Battery={}",
            ram_gb,
            effective_vram.map_or("none".to_string(), |v| format!("{:.1}GB", v)),
            cpu_cores,
            gpu_name.as_deref().unwrap_or("none"),
            tier.display_name(),
            disk_available_gb,
            if on_battery { "yes" } else { "no/AC" },
        );

        Self {
            ram_gb,
            vram_gb: effective_vram,
            cpu_cores,
            gpu_name,
            has_cuda,
            has_metal,
            has_rocm,
            has_directml,
            has_npu: false, // TODO: detect NPU via platform-specific APIs
            tier,
            disk_available_gb,
            on_battery,
        }
    }

    /// Re-detect hardware (e.g. after hardware change or dock/undock).
    /// Returns a new profile with updated values.
    pub fn rescan(&self) -> Self {
        Self::detect()
    }

    /// Check whether the current disk space is sufficient for this tier's models.
    pub fn has_sufficient_disk_space(&self) -> bool {
        self.disk_available_gb >= self.tier.model_footprint_gb()
    }

    /// The maximum tier achievable given current disk space.
    /// May be lower than the hardware tier if disk is constrained.
    pub fn effective_tier(&self) -> DeviceTier {
        if self.has_sufficient_disk_space() {
            self.tier
        } else {
            // Walk down tiers until one fits on disk.
            let tiers = [
                DeviceTier::Ultra,
                DeviceTier::High,
                DeviceTier::Mid,
                DeviceTier::Low,
                DeviceTier::UltraLow,
            ];
            for &t in &tiers {
                if t <= self.tier && self.disk_available_gb >= t.model_footprint_gb() {
                    return t;
                }
            }
            DeviceTier::UltraLow
        }
    }

    /// Summary string suitable for display in a settings panel.
    pub fn summary(&self) -> String {
        format!(
            "{}\nRAM: {:.1} GB | VRAM: {} | CPU: {} cores\nGPU: {}\nDisk free: {:.1} GB | Power: {}",
            self.tier.display_name(),
            self.ram_gb,
            self.vram_gb
                .map_or("N/A".to_string(), |v| format!("{:.1} GB", v)),
            self.cpu_cores,
            self.gpu_name.as_deref().unwrap_or("None detected"),
            self.disk_available_gb,
            if self.on_battery { "Battery" } else { "AC Power" },
        )
    }
}

// ---------------------------------------------------------------------------
// GPU Detection — platform-specific
// ---------------------------------------------------------------------------

struct GpuDetectionResult {
    name: Option<String>,
    vram_gb: Option<f64>,
    has_cuda: bool,
    has_rocm: bool,
}

/// Detect GPU name, VRAM, and acceleration framework availability.
fn detect_gpu() -> GpuDetectionResult {
    // Try NVIDIA first (most common discrete GPU).
    if let Some(nvidia) = detect_nvidia_gpu() {
        return nvidia;
    }

    // Try AMD ROCm.
    if let Some(amd) = detect_amd_gpu() {
        return amd;
    }

    // macOS: try system_profiler for GPU info.
    #[cfg(target_os = "macos")]
    if let Some(mac) = detect_macos_gpu() {
        return mac;
    }

    // Windows: try WMIC / PowerShell for GPU info.
    #[cfg(target_os = "windows")]
    if let Some(win) = detect_windows_gpu() {
        return win;
    }

    // Linux: try lspci as a last resort for GPU name.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    if let Some(linux) = detect_linux_gpu_fallback() {
        return linux;
    }

    GpuDetectionResult {
        name: None,
        vram_gb: None,
        has_cuda: false,
        has_rocm: false,
    }
}

/// Try to detect NVIDIA GPU via `nvidia-smi`.
/// Works on Windows, Linux, and macOS (if CUDA toolkit is installed).
fn detect_nvidia_gpu() -> Option<GpuDetectionResult> {
    // nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits
    // Example output: "NVIDIA GeForce RTX 4070, 12282"
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim().lines().next()?;
    let parts: Vec<&str> = line.splitn(2, ',').collect();

    let name = parts.first().map(|s| s.trim().to_string());
    let vram_mb: Option<f64> = parts.get(1).and_then(|s| s.trim().parse().ok());
    let vram_gb = vram_mb.map(|mb| mb / 1024.0);

    Some(GpuDetectionResult {
        name,
        vram_gb,
        has_cuda: true, // nvidia-smi exists → CUDA is available
        has_rocm: false,
    })
}

/// Try to detect AMD GPU via `rocm-smi`.
fn detect_amd_gpu() -> Option<GpuDetectionResult> {
    // Check if rocm-smi exists.
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("rocm-smi")
        .args(["--showproductname", "--showmeminfo", "vram", "--csv"])
        .output()
        .ok()?;

    if !output.status.success() {
        // Fallback: try rocm-smi without CSV.
        return detect_amd_gpu_fallback();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut name = None;
    let mut vram_gb = None;

    for line in stdout.lines() {
        let line = line.trim();
        if line.contains("Card series") || line.contains("card_series") {
            if let Some(val) = line.split(',').nth(1) {
                let n = val.trim().to_string();
                if !n.is_empty() {
                    name = Some(n);
                }
            }
        }
        // Look for VRAM total in MB.
        if line.contains("Total") && line.contains("vram") {
            if let Some(val) = line.split(',').last() {
                if let Ok(mb) = val.trim().parse::<f64>() {
                    vram_gb = Some(mb / 1024.0);
                }
            }
        }
    }

    if name.is_some() || vram_gb.is_some() {
        Some(GpuDetectionResult {
            name,
            vram_gb,
            has_cuda: false,
            has_rocm: true,
        })
    } else {
        None
    }
}

/// Fallback AMD detection — just check if rocm-smi exists at all.
fn detect_amd_gpu_fallback() -> Option<GpuDetectionResult> {
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("rocm-smi")
        .output()
        .ok()?;

    if output.status.success() {
        Some(GpuDetectionResult {
            name: Some("AMD GPU (ROCm)".to_string()),
            vram_gb: None,
            has_cuda: false,
            has_rocm: true,
        })
    } else {
        None
    }
}

/// macOS: detect GPU via `system_profiler SPDisplaysDataType`.
#[cfg(target_os = "macos")]
fn detect_macos_gpu() -> Option<GpuDetectionResult> {
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut name = None;
    let mut vram_gb = None;

    for line in stdout.lines() {
        let line = line.trim();
        // "Chipset Model: Apple M3 Max" or "Chipset Model: AMD Radeon Pro 5500M"
        if line.starts_with("Chipset Model:") {
            name = line.strip_prefix("Chipset Model:").map(|s| s.trim().to_string());
        }
        // "VRAM (Total): 4 GB" or "VRAM (Dynamic, Max): 48 GB"
        if line.contains("VRAM") && line.contains("GB") {
            if let Some(gb_str) = extract_gb_from_line(line) {
                vram_gb = Some(gb_str);
            }
        }
    }

    // Apple Silicon uses unified memory — VRAM is shared.
    // If we found a chip name containing "Apple M" but no VRAM line, leave vram_gb as None.
    // The caller (detect()) handles Apple Silicon unified memory estimation.

    Some(GpuDetectionResult {
        name,
        vram_gb,
        has_cuda: false,
        has_rocm: false,
    })
}

/// Windows: detect GPU via PowerShell `Get-CimInstance`.
#[cfg(target_os = "windows")]
fn detect_windows_gpu() -> Option<GpuDetectionResult> {
    // PowerShell: Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty Name; Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty AdapterRAM",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return detect_windows_gpu_wmic();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();

    let name = lines.first().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let vram_bytes: Option<f64> = lines.get(1).and_then(|s| s.trim().parse().ok());
    let vram_gb = vram_bytes.map(|b| b / (1024.0 * 1024.0 * 1024.0));

    // Filter out unreasonable values (e.g. 0 or integrated GPUs reporting shared memory).
    let vram_gb = vram_gb.filter(|&v| v > 0.5);

    let has_cuda = name
        .as_ref()
        .is_some_and(|n| n.to_lowercase().contains("nvidia"));

    Some(GpuDetectionResult {
        name,
        vram_gb,
        has_cuda,
        has_rocm: false,
    })
}

/// Windows fallback: detect GPU via `wmic`.
#[cfg(target_os = "windows")]
fn detect_windows_gpu_wmic() -> Option<GpuDetectionResult> {
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "name,AdapterRAM", "/format:csv"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Skip header lines, find first data line.
    for line in stdout.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        // CSV: Node, AdapterRAM, Name
        if parts.len() >= 3 {
            let vram_bytes: Option<f64> = parts.get(1).and_then(|s| s.trim().parse().ok());
            let vram_gb = vram_bytes
                .map(|b| b / (1024.0 * 1024.0 * 1024.0))
                .filter(|&v| v > 0.5);
            let name = parts.get(2).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
            let has_cuda = name
                .as_ref()
                .is_some_and(|n| n.to_lowercase().contains("nvidia"));

            return Some(GpuDetectionResult {
                name,
                vram_gb,
                has_cuda,
                has_rocm: false,
            });
        }
    }
    None
}

/// Linux fallback: detect GPU name via `lspci`.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn detect_linux_gpu_fallback() -> Option<GpuDetectionResult> {
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("lspci")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpu_name = None;
    let mut has_nvidia = false;

    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if lower.contains("vga") || lower.contains("3d") || lower.contains("display") {
            // Extract the part after the description type.
            // "01:00.0 VGA compatible controller: NVIDIA Corporation GA104 [GeForce RTX 3070]"
            if let Some(desc) = line.split(':').nth(2) {
                gpu_name = Some(desc.trim().to_string());
            }
            if lower.contains("nvidia") {
                has_nvidia = true;
            }
            break;
        }
    }

    // Try to read VRAM from sysfs for NVIDIA.
    let vram_gb = if has_nvidia {
        read_nvidia_vram_sysfs()
    } else {
        None
    };

    if gpu_name.is_some() {
        Some(GpuDetectionResult {
            name: gpu_name,
            vram_gb,
            has_cuda: has_nvidia,
            has_rocm: false,
        })
    } else {
        None
    }
}

/// Linux: Try to read NVIDIA VRAM from sysfs.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn read_nvidia_vram_sysfs() -> Option<f64> {
    // /sys/class/drm/card*/device/mem_info_vram_total contains VRAM in bytes.
    let drm_dir = Path::new("/sys/class/drm");
    if !drm_dir.exists() {
        return None;
    }

    for entry in std::fs::read_dir(drm_dir).ok()? {
        let entry = entry.ok()?;
        let vram_path = entry.path().join("device/mem_info_vram_total");
        if vram_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&vram_path) {
                if let Ok(bytes) = contents.trim().parse::<u64>() {
                    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    if gb > 0.5 {
                        return Some(gb);
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Disk Space Detection
// ---------------------------------------------------------------------------

/// Detect available disk space on the volume where DX models would be stored.
///
/// Checks the user's home directory volume (where `~/.dx/models/` lives).
fn detect_disk_space_gb() -> f64 {
    let disks = Disks::new_with_refreshed_list();

    // Try to find the disk that contains the home directory.
    let home = dirs_home();

    let mut best_match: Option<(usize, u64)> = None; // (mount_point_len, available_bytes)

    for disk in disks.list() {
        let mount = disk.mount_point();
        let mount_str = mount.to_string_lossy();

        // Find the disk whose mount point is the longest prefix of the home dir.
        if let Some(ref home) = home {
            if home.starts_with(mount_str.as_ref()) {
                let len = mount_str.len();
                if best_match.is_none() || len > best_match.unwrap().0 {
                    best_match = Some((len, disk.available_space()));
                }
            }
        }
    }

    // Fallback: if we couldn't match home, use the largest disk.
    let available_bytes = best_match
        .map(|(_, bytes)| bytes)
        .or_else(|| {
            disks
                .list()
                .iter()
                .map(|d| d.available_space())
                .max()
        })
        .unwrap_or(0);

    available_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

/// Get the user's home directory path as a string.
fn dirs_home() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok()
    }
}

// ---------------------------------------------------------------------------
// Battery / Power State Detection
// ---------------------------------------------------------------------------

/// Detect whether the device is currently running on battery power.
fn detect_battery_state() -> bool {
    #[cfg(target_os = "macos")]
    {
        detect_battery_macos()
    }
    #[cfg(target_os = "windows")]
    {
        detect_battery_windows()
    }
    #[cfg(target_os = "linux")]
    {
        detect_battery_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/// macOS: check battery via `pmset -g batt`.
#[cfg(target_os = "macos")]
fn detect_battery_macos() -> bool {
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // If output contains "Battery Power" it's on battery.
            // If it contains "AC Power" it's plugged in.
            stdout.contains("Battery Power")
        }
        _ => false,
    }
}

/// Windows: check battery via PowerShell.
#[cfg(target_os = "windows")]
fn detect_battery_windows() -> bool {
    #[allow(clippy::disallowed_methods)]
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_Battery).BatteryStatus",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let status = stdout.trim();
            // BatteryStatus: 1 = discharging (on battery), 2 = AC power
            // If command returns nothing (no battery), assume AC.
            status == "1"
        }
        _ => false,
    }
}

/// Linux: check battery via /sys/class/power_supply/.
#[cfg(target_os = "linux")]
fn detect_battery_linux() -> bool {
    let power_dir = Path::new("/sys/class/power_supply");
    if !power_dir.exists() {
        return false;
    }

    if let Ok(entries) = std::fs::read_dir(power_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Look for BAT0, BAT1, etc.
            if name_str.starts_with("BAT") {
                let status_path = entry.path().join("status");
                if let Ok(status) = std::fs::read_to_string(&status_path) {
                    let status = status.trim().to_lowercase();
                    // "discharging" = on battery, "charging" or "full" = on AC
                    if status == "discharging" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a GB value from a line like "VRAM (Total): 4 GB".
#[cfg(target_os = "macos")]
fn extract_gb_from_line(line: &str) -> Option<f64> {
    // Find a number followed by "GB" (case-insensitive).
    let lower = line.to_lowercase();
    if let Some(gb_pos) = lower.find("gb") {
        // Walk backwards from "gb" to find the number.
        let before = &line[..gb_pos].trim();
        // Take the last whitespace-separated token.
        if let Some(num_str) = before.split_whitespace().last() {
            return num_str.parse().ok();
        }
    }
    None
}

/// Model recommendation for a specific device tier and purpose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub purpose: ModelPurpose,
    pub model_name: String,
    pub quantization: String,
    pub ram_required_mb: u64,
    pub disk_required_mb: u64,
    pub download_url: Option<String>,
}

/// What a local model is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelPurpose {
    Llm,
    CodePrediction,
    ProsePrediction,
    Grammar,
    SpeechToText,
    TextToSpeech,
    ImageGeneration,
    ThreeDGeneration,
    Vision,
    Embeddings,
}

impl ModelPurpose {
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelPurpose::Llm => "Language Model",
            ModelPurpose::CodePrediction => "Code Prediction",
            ModelPurpose::ProsePrediction => "Prose Prediction",
            ModelPurpose::Grammar => "Grammar",
            ModelPurpose::SpeechToText => "Speech-to-Text",
            ModelPurpose::TextToSpeech => "Text-to-Speech",
            ModelPurpose::ImageGeneration => "Image Generation",
            ModelPurpose::ThreeDGeneration => "3D Generation",
            ModelPurpose::Vision => "Vision",
            ModelPurpose::Embeddings => "Embeddings",
        }
    }
}

/// Get model recommendations for a given device tier.
pub fn recommended_models(tier: DeviceTier) -> Vec<ModelRecommendation> {
    match tier {
        DeviceTier::UltraLow => vec![
            ModelRecommendation {
                purpose: ModelPurpose::Llm,
                model_name: "SmolLM2-360M".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 300,
                disk_required_mb: 200,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::CodePrediction,
                model_name: "SmolLM2-135M".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 150,
                disk_required_mb: 100,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::SpeechToText,
                model_name: "Whisper Tiny.en".into(),
                quantization: "f16".into(),
                ram_required_mb: 100,
                disk_required_mb: 75,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::TextToSpeech,
                model_name: "Piper tiny.en".into(),
                quantization: "onnx".into(),
                ram_required_mb: 15,
                disk_required_mb: 15,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Embeddings,
                model_name: "all-MiniLM-L6-v2".into(),
                quantization: "f32".into(),
                ram_required_mb: 50,
                disk_required_mb: 23,
                download_url: None,
            },
        ],
        DeviceTier::Low => vec![
            ModelRecommendation {
                purpose: ModelPurpose::Llm,
                model_name: "Qwen3-0.6B".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 500,
                disk_required_mb: 400,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::CodePrediction,
                model_name: "SmolLM2-360M".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 300,
                disk_required_mb: 200,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::SpeechToText,
                model_name: "Whisper Tiny.en".into(),
                quantization: "f16".into(),
                ram_required_mb: 100,
                disk_required_mb: 75,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::TextToSpeech,
                model_name: "Piper medium.en".into(),
                quantization: "onnx".into(),
                ram_required_mb: 65,
                disk_required_mb: 50,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Embeddings,
                model_name: "all-MiniLM-L6-v2".into(),
                quantization: "f32".into(),
                ram_required_mb: 50,
                disk_required_mb: 23,
                download_url: None,
            },
        ],
        DeviceTier::Mid => vec![
            ModelRecommendation {
                purpose: ModelPurpose::Llm,
                model_name: "Qwen2.5-3B-Instruct".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 2000,
                disk_required_mb: 1800,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::CodePrediction,
                model_name: "Qwen2.5-Coder-1.5B".into(),
                quantization: "Q5_K_M".into(),
                ram_required_mb: 1200,
                disk_required_mb: 1000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::ProsePrediction,
                model_name: "SmolLM2-1.7B".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 1200,
                disk_required_mb: 1000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::SpeechToText,
                model_name: "Whisper Base.en".into(),
                quantization: "f16".into(),
                ram_required_mb: 200,
                disk_required_mb: 142,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::TextToSpeech,
                model_name: "Piper high.en + Kokoro".into(),
                quantization: "onnx".into(),
                ram_required_mb: 100,
                disk_required_mb: 80,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Embeddings,
                model_name: "all-MiniLM-L6-v2".into(),
                quantization: "f32".into(),
                ram_required_mb: 50,
                disk_required_mb: 23,
                download_url: None,
            },
        ],
        DeviceTier::High => vec![
            ModelRecommendation {
                purpose: ModelPurpose::Llm,
                model_name: "Mistral-7B-Instruct".into(),
                quantization: "Q5_K_M".into(),
                ram_required_mb: 6500,
                disk_required_mb: 5100,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Grammar,
                model_name: "SmolLM3-3B".into(),
                quantization: "Q5_K_M".into(),
                ram_required_mb: 2500,
                disk_required_mb: 2000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::CodePrediction,
                model_name: "Qwen2.5-Coder-7B (Zeta)".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 4500,
                disk_required_mb: 3800,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::ProsePrediction,
                model_name: "Qwen2.5-3B-Instruct".into(),
                quantization: "Q5_K_M".into(),
                ram_required_mb: 2500,
                disk_required_mb: 2000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::SpeechToText,
                model_name: "Whisper Small.en".into(),
                quantization: "f16".into(),
                ram_required_mb: 400,
                disk_required_mb: 244,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::TextToSpeech,
                model_name: "Chatterbox-Turbo".into(),
                quantization: "f16".into(),
                ram_required_mb: 500,
                disk_required_mb: 400,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::ImageGeneration,
                model_name: "SDXL Turbo".into(),
                quantization: "Q4".into(),
                ram_required_mb: 3500,
                disk_required_mb: 2800,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Embeddings,
                model_name: "all-MiniLM-L6-v2".into(),
                quantization: "f32".into(),
                ram_required_mb: 50,
                disk_required_mb: 23,
                download_url: None,
            },
        ],
        DeviceTier::Ultra => vec![
            ModelRecommendation {
                purpose: ModelPurpose::Llm,
                model_name: "Qwen2.5-72B".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 40000,
                disk_required_mb: 38000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Grammar,
                model_name: "Qwen2.5-14B-Instruct".into(),
                quantization: "Q5_K_M".into(),
                ram_required_mb: 10000,
                disk_required_mb: 9000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::CodePrediction,
                model_name: "Qwen2.5-Coder-32B".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 20000,
                disk_required_mb: 18000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::ProsePrediction,
                model_name: "Mistral-7B-Instruct".into(),
                quantization: "Q6_K".into(),
                ram_required_mb: 6000,
                disk_required_mb: 5500,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::SpeechToText,
                model_name: "Whisper Large-v3".into(),
                quantization: "f16".into(),
                ram_required_mb: 3000,
                disk_required_mb: 1500,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::TextToSpeech,
                model_name: "Chatterbox-Turbo + voice cloning".into(),
                quantization: "f16".into(),
                ram_required_mb: 1000,
                disk_required_mb: 800,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::ImageGeneration,
                model_name: "Flux.1 Dev".into(),
                quantization: "f16".into(),
                ram_required_mb: 12000,
                disk_required_mb: 11000,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Vision,
                model_name: "LLaVA-1.5-7B".into(),
                quantization: "Q4_K_M".into(),
                ram_required_mb: 4500,
                disk_required_mb: 3800,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::ThreeDGeneration,
                model_name: "TripoSR".into(),
                quantization: "f16".into(),
                ram_required_mb: 3000,
                disk_required_mb: 2500,
                download_url: None,
            },
            ModelRecommendation {
                purpose: ModelPurpose::Embeddings,
                model_name: "all-MiniLM-L6-v2".into(),
                quantization: "f32".into(),
                ram_required_mb: 50,
                disk_required_mb: 23,
                download_url: None,
            },
        ],
    }
}
