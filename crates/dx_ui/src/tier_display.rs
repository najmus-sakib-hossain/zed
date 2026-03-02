//! TierDisplay — Hardware tier display with manual override option.
//!
//! Part 15 UI: Shows the detected DeviceTier, hardware summary,
//! and lets the user manually override the tier classification.

use dx_core::device_tier::DeviceTier;
use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone)]
pub struct HardwareSummary {
    pub cpu_name: String,
    pub ram_gb: f64,
    pub gpu_name: Option<String>,
    pub gpu_vram_mb: Option<u64>,
    pub has_npu: bool,
    pub battery_present: bool,
    pub disk_free_gb: f64,
}

impl Default for HardwareSummary {
    fn default() -> Self {
        Self {
            cpu_name: "Unknown CPU".into(),
            ram_gb: 0.0,
            gpu_name: None,
            gpu_vram_mb: None,
            has_npu: false,
            battery_present: false,
            disk_free_gb: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TierDisplayEvent {
    TierOverridden(DeviceTier),
    TierResetToAuto,
}

pub struct TierDisplay {
    focus_handle: FocusHandle,
    detected_tier: DeviceTier,
    override_tier: Option<DeviceTier>,
    hardware_summary: HardwareSummary,
    show_details: bool,
}

impl TierDisplay {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            detected_tier: DeviceTier::Mid,
            override_tier: None,
            hardware_summary: HardwareSummary::default(),
            show_details: false,
        }
    }

    pub fn effective_tier(&self) -> DeviceTier {
        self.override_tier.unwrap_or(self.detected_tier)
    }

    pub fn set_detected_tier(&mut self, tier: DeviceTier, cx: &mut ViewContext<Self>) {
        self.detected_tier = tier;
        cx.notify();
    }

    pub fn set_hardware_summary(
        &mut self,
        summary: HardwareSummary,
        cx: &mut ViewContext<Self>,
    ) {
        self.hardware_summary = summary;
        cx.notify();
    }

    pub fn override_tier(&mut self, tier: DeviceTier, cx: &mut ViewContext<Self>) {
        self.override_tier = Some(tier);
        cx.emit(TierDisplayEvent::TierOverridden(tier));
        cx.notify();
    }

    pub fn reset_to_auto(&mut self, cx: &mut ViewContext<Self>) {
        self.override_tier = None;
        cx.emit(TierDisplayEvent::TierResetToAuto);
        cx.notify();
    }

    pub fn toggle_details(&mut self, cx: &mut ViewContext<Self>) {
        self.show_details = !self.show_details;
        cx.notify();
    }

    fn tier_label(tier: DeviceTier) -> &'static str {
        match tier {
            DeviceTier::Potato => "🥔 Potato",
            DeviceTier::Low => "📱 Low",
            DeviceTier::Mid => "💻 Mid",
            DeviceTier::High => "🖥️ High",
            DeviceTier::Ultra => "🚀 Ultra",
        }
    }

    fn tier_description(tier: DeviceTier) -> &'static str {
        match tier {
            DeviceTier::Potato => "Cloud-only mode. Local models disabled.",
            DeviceTier::Low => "Tiny local models (≤1B). Prefer cloud.",
            DeviceTier::Mid => "Small local models (≤7B). Balanced cloud/local.",
            DeviceTier::High => "Medium local models (≤30B). Local-first.",
            DeviceTier::Ultra => "Large local models (70B+). Full local capability.",
        }
    }

    fn all_tiers() -> &'static [DeviceTier] {
        &[
            DeviceTier::Potato,
            DeviceTier::Low,
            DeviceTier::Mid,
            DeviceTier::High,
            DeviceTier::Ultra,
        ]
    }
}

impl gpui::EventEmitter<TierDisplayEvent> for TierDisplay {}

impl Focusable for TierDisplay {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TierDisplay {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let effective = self.effective_tier();
        let detected = self.detected_tier;
        let is_overridden = self.override_tier.is_some();
        let show_details = self.show_details;
        let summary = self.hardware_summary.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            // Current tier display
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(SharedString::from("Hardware Tier"))
                            )
                            .child(
                                div()
                                    .text_size(px(20.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(SharedString::from(Self::tier_label(effective)))
                            )
                            .when(is_overridden, |d| {
                                d.child(
                                    div()
                                        .text_xs()
                                        .child(SharedString::from(format!(
                                            "Auto-detected: {}",
                                            Self::tier_label(detected)
                                        )))
                                )
                            })
                    )
                    // Details toggle
                    .child(
                        div()
                            .cursor_pointer()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .border_1()
                            .text_xs()
                            .child(SharedString::from(
                                if show_details { "Hide Details" } else { "Show Details" }
                            ))
                    )
            )
            // Tier description
            .child(
                div()
                    .text_sm()
                    .child(SharedString::from(Self::tier_description(effective)))
            )
            // Hardware details (expandable)
            .when(show_details, |container| {
                container
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .p(px(12.0))
                            .rounded(px(8.0))
                            .border_1()
                            .child(detail_row("CPU", &summary.cpu_name))
                            .child(detail_row("RAM", &format!("{:.1} GB", summary.ram_gb)))
                            .child(detail_row(
                                "GPU",
                                summary.gpu_name.as_deref().unwrap_or("None"),
                            ))
                            .child(detail_row(
                                "VRAM",
                                &summary
                                    .gpu_vram_mb
                                    .map(|v| format!("{} MB", v))
                                    .unwrap_or_else(|| "N/A".into()),
                            ))
                            .child(detail_row(
                                "NPU",
                                if summary.has_npu { "Yes" } else { "No" },
                            ))
                            .child(detail_row(
                                "Disk Free",
                                &format!("{:.1} GB", summary.disk_free_gb),
                            ))
                    )
            })
            // Manual tier override buttons
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(SharedString::from("Override Tier"))
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(4.0))
                            .children(Self::all_tiers().iter().map(|&tier| {
                                let is_active = tier == effective;
                                div()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .border_1()
                                    .cursor_pointer()
                                    .text_xs()
                                    .when(is_active, |d| {
                                        d.font_weight(gpui::FontWeight::BOLD)
                                    })
                                    .child(SharedString::from(Self::tier_label(tier)))
                            }))
                    )
                    .when(is_overridden, |d| {
                        d.child(
                            div()
                                .cursor_pointer()
                                .text_xs()
                                .pt(px(4.0))
                                .child(SharedString::from("Reset to Auto-Detect"))
                        )
                    })
            )
    }
}

fn detail_row(label: &str, value: &str) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(SharedString::from(label.to_string()))
        )
        .child(
            div()
                .text_xs()
                .child(SharedString::from(value.to_string()))
        )
}
