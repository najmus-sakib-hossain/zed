//! NPU/TPU detection — identifies specialized AI accelerators.
//!
//! Detects Neural Processing Units (NPUs) on supported hardware:
//! - Intel NPU (Meteor Lake, Arrow Lake)
//! - Qualcomm Hexagon NPU (Snapdragon X Elite)
//! - Apple Neural Engine (M1+)
//! - AMD XDNA (Ryzen AI)
//! - Google TPU (cloud)

use serde::{Deserialize, Serialize};

/// Detected NPU/AI accelerator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpuInfo {
    /// NPU vendor.
    pub vendor: NpuVendor,
    /// Human-readable name.
    pub name: String,
    /// Estimated TOPS (Tera Operations Per Second).
    pub tops: Option<f64>,
    /// Whether the NPU driver is available.
    pub driver_available: bool,
    /// Whether DirectML or CoreML can target this NPU.
    pub ml_framework_support: Vec<String>,
}

/// NPU vendor classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpuVendor {
    Intel,
    Qualcomm,
    Apple,
    Amd,
    Google,
    Unknown,
}

impl NpuVendor {
    pub fn display_name(&self) -> &'static str {
        match self {
            NpuVendor::Intel => "Intel NPU",
            NpuVendor::Qualcomm => "Qualcomm Hexagon",
            NpuVendor::Apple => "Apple Neural Engine",
            NpuVendor::Amd => "AMD XDNA",
            NpuVendor::Google => "Google TPU",
            NpuVendor::Unknown => "Unknown NPU",
        }
    }
}

/// Detect NPU/AI accelerators on the current system.
pub fn detect_npu() -> Vec<NpuInfo> {
    let mut npus = Vec::new();

    // Apple Neural Engine detection (macOS only)
    #[cfg(target_os = "macos")]
    {
        // All Apple Silicon Macs have the Neural Engine
        if is_apple_silicon() {
            npus.push(NpuInfo {
                vendor: NpuVendor::Apple,
                name: "Apple Neural Engine".to_string(),
                tops: Some(38.0), // M3 Pro ~38 TOPS, M4 ~38 TOPS
                driver_available: true,
                ml_framework_support: vec![
                    "CoreML".to_string(),
                    "Metal Performance Shaders".to_string(),
                ],
            });
        }
    }

    // Windows NPU detection
    #[cfg(target_os = "windows")]
    {
        // Intel NPU — check for Intel AI Boost (Meteor Lake+)
        if let Some(npu) = detect_intel_npu_windows() {
            npus.push(npu);
        }

        // Qualcomm NPU — Snapdragon X Elite
        if let Some(npu) = detect_qualcomm_npu_windows() {
            npus.push(npu);
        }

        // AMD XDNA
        if let Some(npu) = detect_amd_npu_windows() {
            npus.push(npu);
        }
    }

    // Linux NPU detection
    #[cfg(target_os = "linux")]
    {
        if let Some(npu) = detect_intel_npu_linux() {
            npus.push(npu);
        }
    }

    if npus.is_empty() {
        log::debug!("No NPU/AI accelerators detected");
    } else {
        for npu in &npus {
            log::info!(
                "Detected NPU: {} ({}, {:.1} TOPS)",
                npu.name,
                npu.vendor.display_name(),
                npu.tops.unwrap_or(0.0)
            );
        }
    }

    npus
}

#[cfg(target_os = "macos")]
fn is_apple_silicon() -> bool {
    // Check CPU brand via sysctl
    std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|brand| brand.contains("Apple"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn detect_intel_npu_windows() -> Option<NpuInfo> {
    // Check for Intel NPU driver via device manager
    // Intel NPU shows up as "Intel(R) AI Boost" in Device Manager
    let output = std::process::Command::new("powershell")
        .args([
            "-Command",
            "Get-WmiObject Win32_PnPEntity | Where-Object { $_.Name -match 'Intel.*NPU|Intel.*AI Boost' } | Select-Object -First 1 Name",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    if stdout.contains("Intel") {
        Some(NpuInfo {
            vendor: NpuVendor::Intel,
            name: "Intel AI Boost NPU".to_string(),
            tops: Some(11.0), // Meteor Lake ~11 TOPS
            driver_available: true,
            ml_framework_support: vec!["DirectML".to_string(), "OpenVINO".to_string()],
        })
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn detect_qualcomm_npu_windows() -> Option<NpuInfo> {
    let output = std::process::Command::new("powershell")
        .args([
            "-Command",
            "Get-WmiObject Win32_PnPEntity | Where-Object { $_.Name -match 'Qualcomm.*NPU|Hexagon' } | Select-Object -First 1 Name",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    if stdout.contains("Qualcomm") || stdout.contains("Hexagon") {
        Some(NpuInfo {
            vendor: NpuVendor::Qualcomm,
            name: "Qualcomm Hexagon NPU".to_string(),
            tops: Some(45.0), // Snapdragon X Elite ~45 TOPS
            driver_available: true,
            ml_framework_support: vec!["DirectML".to_string(), "QNN".to_string()],
        })
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn detect_amd_npu_windows() -> Option<NpuInfo> {
    let output = std::process::Command::new("powershell")
        .args([
            "-Command",
            "Get-WmiObject Win32_PnPEntity | Where-Object { $_.Name -match 'AMD.*NPU|XDNA' } | Select-Object -First 1 Name",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    if stdout.contains("AMD") || stdout.contains("XDNA") {
        Some(NpuInfo {
            vendor: NpuVendor::Amd,
            name: "AMD XDNA NPU".to_string(),
            tops: Some(16.0), // Ryzen AI ~16 TOPS
            driver_available: true,
            ml_framework_support: vec!["DirectML".to_string(), "ROCm".to_string()],
        })
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn detect_intel_npu_linux() -> Option<NpuInfo> {
    // Check for Intel NPU driver device node
    let npu_path = std::path::Path::new("/dev/accel/accel0");
    if npu_path.exists() {
        return Some(NpuInfo {
            vendor: NpuVendor::Intel,
            name: "Intel AI Boost NPU".to_string(),
            tops: Some(11.0),
            driver_available: true,
            ml_framework_support: vec!["OpenVINO".to_string()],
        });
    }
    None
}
