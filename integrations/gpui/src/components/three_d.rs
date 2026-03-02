//! 🧊 3D Rendering Panel
//!
//! Stack: wgpu (GPU API) + rend3 (PBR renderer) + glam (math) + naga (shaders)
//! Pipeline: Render 3D scene to off-screen wgpu texture → copy pixels to GPUI image

use gpui::*;
use super::video::card;

pub struct ThreeDPanel {
    status: String,
    features: Vec<&'static str>,
}

impl ThreeDPanel {
    pub fn new() -> Self {
        Self {
            status: "Ready".to_string(),
            features: vec![
                "wgpu — Vulkan/Metal/DX12/OpenGL (pure Rust)",
                "rend3 — PBR, shadows, skyboxes, tonemapping",
                "glam — fast 3D math (pure Rust)",
                "naga — WGSL/SPIR-V/MSL/HLSL shaders (pure Rust)",
            ],
        }
    }

    pub fn render_card(&self) -> impl IntoElement {
        let items = self
            .features
            .iter()
            .map(|f| {
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(0xBBBBBB))
                    .child(format!("  🧊 {}", f))
            })
            .collect::<Vec<_>>();

        card(
            "🧊 3D Rendering",
            &self.status,
            rgb(0x3498db),
            "Off-screen wgpu texture → GPUI image element",
            items,
        )
    }
}
