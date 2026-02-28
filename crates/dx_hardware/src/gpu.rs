//! GPU capability detection.

use serde::{Deserialize, Serialize};

/// GPU vendor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Unknown,
}

/// GPU capabilities for model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapability {
    /// GPU vendor.
    pub vendor: GpuVendor,
    /// GPU model name.
    pub model_name: String,
    /// Video RAM in bytes.
    pub vram_bytes: u64,
    /// Whether CUDA is available.
    pub cuda_available: bool,
    /// CUDA compute capability (e.g., 8.6).
    pub cuda_compute_capability: Option<(u32, u32)>,
    /// Whether ROCm/HIP is available.
    pub rocm_available: bool,
    /// Whether Metal is available (macOS).
    pub metal_available: bool,
    /// Whether Vulkan compute is available.
    pub vulkan_available: bool,
}

impl GpuCapability {
    /// Detect GPU capabilities. Placeholder implementation.
    pub fn detect() -> Self {
        Self {
            vendor: GpuVendor::Unknown,
            model_name: "Unknown GPU".into(),
            vram_bytes: 0,
            cuda_available: false,
            cuda_compute_capability: None,
            rocm_available: false,
            metal_available: cfg!(target_os = "macos"),
            vulkan_available: false,
        }
    }

    /// No usable GPU detected.
    pub fn is_cpu_only(&self) -> bool {
        self.vram_bytes == 0
            && !self.cuda_available
            && !self.rocm_available
            && !self.metal_available
    }

    /// Whether this GPU can run GGUF models efficiently.
    pub fn supports_gguf_offload(&self) -> bool {
        self.cuda_available || self.metal_available || self.rocm_available
    }

    /// Recommended maximum model size in parameters.
    pub fn max_model_params(&self) -> u64 {
        let vram_gb = self.vram_bytes / 1_073_741_824;
        match vram_gb {
            0 => 0,                     // CPU only
            1..=3 => 1_000_000_000,     // 1B params (GGUF Q4)
            4..=7 => 7_000_000_000,     // 7B
            8..=11 => 13_000_000_000,   // 13B
            12..=23 => 34_000_000_000,  // 34B
            _ => 70_000_000_000,        // 70B
        }
    }

    /// Summary string.
    pub fn summary(&self) -> String {
        if self.is_cpu_only() {
            "CPU only".into()
        } else {
            format!(
                "{:?} {} ({:.1} GB VRAM)",
                self.vendor,
                self.model_name,
                self.vram_bytes as f64 / 1_073_741_824.0
            )
        }
    }
}

impl Default for GpuCapability {
    fn default() -> Self {
        Self::detect()
    }
}
