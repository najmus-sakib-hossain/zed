//! dx_hardware — Hardware profiling, device tier detection, and resource monitoring.
//!
//! Detects CPU, GPU, RAM, and disk capabilities to determine the optimal
//! model selection for local inference. Monitors resources in real time
//! to dynamically swap models when the system is under pressure.

pub mod gpu;
pub mod model_selector;
pub mod monitor;
pub mod profiler;

pub use gpu::GpuCapability;
pub use model_selector::{ModelRecommendation, ModelSelector};
pub use monitor::ResourceMonitor;
pub use profiler::HardwareProfile;

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
