//! DX Media — Universal media generation engine.
//!
//! Provides concrete implementations of `MediaProvider` for all supported
//! image, video, audio, music, 3D, and document generation providers.

mod document_generator;
mod fal_ai;
mod openai_image;
mod orchestrator;
mod stability_ai;

pub use document_generator::*;
pub use fal_ai::*;
pub use openai_image::*;
pub use orchestrator::*;
pub use stability_ai::*;

use gpui::App;

/// Initialize the media generation subsystem.
pub fn init(_cx: &mut App) {
    log::info!("DX Media engine initialized");
}
