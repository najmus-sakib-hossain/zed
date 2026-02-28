//! DX Core — Foundation types, traits, and provider abstractions for the Universal AI Platform.
//!
//! This crate defines the shared interfaces used across all DX subsystems:
//! - Universe A (Language Intelligence): 100+ LLM providers
//! - Universe B (Media Generation): 50+ media providers
//! - Hardware tier classification and real-time detection
//! - Cost tracking, rate limiting, fallback chains
//! - Unified provider registry
//! - Configuration persistence (`~/.dx/dx_config.json`)

mod config;
mod cost;
mod device_tier;
mod llm_provider;
mod media_provider;
mod mood;
mod profile;
mod provider_registry;
mod rate_limiter;
mod session;
mod tts_provider;

pub use config::*;
pub use cost::*;
pub use device_tier::*;
pub use llm_provider::*;
pub use media_provider::*;
pub use mood::*;
pub use profile::*;
pub use provider_registry::*;
pub use rate_limiter::*;
pub use session::*;
pub use tts_provider::*;

use gpui::App;

/// Maximum age of a cached hardware profile before re-detection is triggered (7 days).
const HARDWARE_RESCAN_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);

/// Initialize DX core systems.
///
/// This performs first-launch setup if needed:
/// 1. Loads or creates `~/.dx/dx_config.json`
/// 2. Detects hardware if no cached profile exists (or if stale)
/// 3. Logs the detected tier and model recommendations
///
/// This is a synchronous, fast-path init. Heavy work (model downloads, provider
/// health checks) should be kicked off asynchronously after this returns.
pub fn init(_cx: &mut App) {
    log::info!("DX Core initializing...");

    match init_config_and_hardware() {
        Ok((config, profile)) => {
            let tier = config
                .effective_tier()
                .unwrap_or(DeviceTier::UltraLow);

            log::info!("DX Core initialized — {}", tier.display_name());
            log::info!("Hardware: {}", profile.summary());

            if !profile.has_sufficient_disk_space() {
                log::warn!(
                    "Insufficient disk space for {} models ({:.1} GB available, {:.1} GB needed). \
                     Effective tier downgraded to {}.",
                    tier.display_name(),
                    profile.disk_available_gb,
                    tier.model_footprint_gb(),
                    profile.effective_tier().display_name(),
                );
            }

            let models = recommended_models(profile.effective_tier());
            let total_disk_mb: u64 = models.iter().map(|m| m.disk_required_mb).sum();
            log::info!(
                "Recommended {} models for {} (~{} MB on disk)",
                models.len(),
                profile.effective_tier().display_name(),
                total_disk_mb,
            );
        }
        Err(e) => {
            log::error!("DX Core init failed to load config: {:?}", e);
            log::info!("DX Core initialized with defaults (config unavailable)");
        }
    }
}

/// Load config and detect hardware if needed. Returns the config and hardware profile.
fn init_config_and_hardware() -> anyhow::Result<(DxConfig, HardwareProfile)> {
    let mut config = DxConfig::load().unwrap_or_else(|e| {
        log::warn!("Failed to load DX config, using defaults: {:?}", e);
        DxConfig::default()
    });

    let profile = if config.needs_hardware_rescan(HARDWARE_RESCAN_MAX_AGE) {
        log::info!("Detecting hardware (first launch or stale cache)...");
        let profile = HardwareProfile::detect();

        // Persist the detected profile so we don't re-detect next launch.
        if let Err(e) = config.update_hardware_profile(profile.clone()) {
            log::warn!("Failed to persist hardware profile: {:?}", e);
        }

        profile
    } else {
        log::info!("Using cached hardware profile");
        config
            .hardware
            .as_ref()
            .map(|c| c.profile.clone())
            .unwrap_or_else(|| HardwareProfile::detect())
    };

    Ok((config, profile))
}
