//! dx_hardware — Hardware profiling, device tier detection, and resource monitoring.
//!
//! Detects CPU, GPU, RAM, NPU, and disk capabilities to determine the optimal
//! model selection for local inference. Monitors resources in real time
//! to dynamically swap models when the system is under pressure.
//!
//! ## Modules
//!
//! - **profiler** — Full hardware detection (CPU, RAM, GPU, disk)
//! - **gpu** — GPU capability detection (vendor, VRAM, compute API)
//! - **npu** — NPU/AI accelerator detection (Intel, Qualcomm, Apple, AMD)
//! - **model_selector** — Tier-based model recommendations
//! - **model_swapper** — Dynamic model swapping under resource pressure
//! - **monitor** — Real-time CPU/RAM/GPU monitoring
//! - **system_analysis** — AI workload scoring and bottleneck detection

pub mod gpu;
pub mod model_selector;
pub mod model_swapper;
pub mod monitor;
pub mod npu;
pub mod profiler;
pub mod system_analysis;

pub use gpu::GpuCapability;
pub use model_selector::{ModelRecommendation, ModelSelector};
pub use model_swapper::{ModelSwapper, PowerState, ResourceSnapshot, SwapDecision};
pub use monitor::ResourceMonitor;
pub use npu::{NpuInfo, NpuVendor};
pub use profiler::HardwareProfile;
pub use system_analysis::{Bottleneck, WorkloadScore};

/// Re-export DeviceTier from dx_core.
pub use dx_core::DeviceTier;

/// Profile the current hardware and return the detected tier.
pub fn detect_tier() -> DeviceTier {
    DeviceTier::detect()
}

/// Profile the current machine fully.
pub fn profile_hardware() -> HardwareProfile {
    HardwareProfile::detect()
}

/// Detect NPU/AI accelerators on the system.
pub fn detect_npus() -> Vec<NpuInfo> {
    npu::detect_npu()
}

/// Analyze the system for AI workload readiness.
pub fn analyze_workload() -> WorkloadScore {
    let profile = profile_hardware();
    let has_npu = !detect_npus().is_empty();
    let tier = detect_tier();

    system_analysis::analyze_system(
        profile.ram_gb(),
        profile.vram_gb(),
        profile.cpu_cores(),
        true, // Assume SSD for modern systems
        has_npu,
        tier,
    )
}
