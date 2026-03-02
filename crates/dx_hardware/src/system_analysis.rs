//! System analysis — AI workload scoring and bottleneck detection.
//!
//! Analyzes the full system to determine optimal workload distribution:
//! which models to load, where to run inference, and when to swap.

use dx_core::DeviceTier;
use serde::{Deserialize, Serialize};

/// AI workload score result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadScore {
    /// Overall score (0-100). Higher = better for AI workloads.
    pub overall: u32,
    /// CPU score — affects tokenization, preprocessing.
    pub cpu_score: u32,
    /// Memory score — affects model loading capacity.
    pub memory_score: u32,
    /// GPU score — affects inference speed.
    pub gpu_score: u32,
    /// Storage score — affects model loading time.
    pub storage_score: u32,
    /// NPU score — bonus for AI accelerators.
    pub npu_score: u32,
    /// Detected bottleneck (the weakest link).
    pub bottleneck: Bottleneck,
    /// Recommended actions to improve performance.
    pub recommendations: Vec<String>,
}

/// System bottleneck that limits AI performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bottleneck {
    /// CPU is the bottleneck (slow tokenization, preprocessing).
    Cpu,
    /// RAM is the bottleneck (can't fit models in memory).
    Memory,
    /// GPU/VRAM is the bottleneck (slow inference or no GPU).
    Gpu,
    /// Storage is the bottleneck (slow model loading from disk).
    Storage,
    /// Thermal throttling detected.
    Thermal,
    /// No clear bottleneck — system is balanced.
    Balanced,
}

impl Bottleneck {
    pub fn display_name(&self) -> &'static str {
        match self {
            Bottleneck::Cpu => "CPU",
            Bottleneck::Memory => "Memory",
            Bottleneck::Gpu => "GPU / VRAM",
            Bottleneck::Storage => "Storage",
            Bottleneck::Thermal => "Thermal",
            Bottleneck::Balanced => "Balanced (no bottleneck)",
        }
    }
}

/// Analyze the system for AI workload readiness.
pub fn analyze_system(
    ram_gb: f64,
    vram_gb: f64,
    cpu_cores: u32,
    has_ssd: bool,
    has_npu: bool,
    tier: DeviceTier,
) -> WorkloadScore {
    let cpu_score = score_cpu(cpu_cores);
    let memory_score = score_memory(ram_gb);
    let gpu_score = score_gpu(vram_gb);
    let storage_score = if has_ssd { 80 } else { 30 };
    let npu_score = if has_npu { 20 } else { 0 };

    let overall = ((cpu_score + memory_score + gpu_score + storage_score + npu_score) as f64 / 5.0)
        .min(100.0) as u32;

    // Detect bottleneck
    let min_score = cpu_score.min(memory_score).min(gpu_score).min(storage_score);
    let bottleneck = if min_score == cpu_score && cpu_score < 40 {
        Bottleneck::Cpu
    } else if min_score == memory_score && memory_score < 40 {
        Bottleneck::Memory
    } else if min_score == gpu_score && gpu_score < 40 {
        Bottleneck::Gpu
    } else if min_score == storage_score && storage_score < 40 {
        Bottleneck::Storage
    } else {
        Bottleneck::Balanced
    };

    let recommendations = generate_recommendations(tier, &bottleneck, ram_gb, vram_gb);

    WorkloadScore {
        overall,
        cpu_score,
        memory_score,
        gpu_score,
        storage_score,
        npu_score,
        bottleneck,
        recommendations,
    }
}

fn score_cpu(cores: u32) -> u32 {
    match cores {
        0..=2 => 20,
        3..=4 => 40,
        5..=8 => 60,
        9..=16 => 80,
        _ => 100,
    }
}

fn score_memory(ram_gb: f64) -> u32 {
    if ram_gb < 4.0 {
        15
    } else if ram_gb < 8.0 {
        35
    } else if ram_gb < 16.0 {
        55
    } else if ram_gb < 32.0 {
        75
    } else if ram_gb < 64.0 {
        90
    } else {
        100
    }
}

fn score_gpu(vram_gb: f64) -> u32 {
    if vram_gb < 1.0 {
        10
    } else if vram_gb < 4.0 {
        30
    } else if vram_gb < 8.0 {
        55
    } else if vram_gb < 16.0 {
        75
    } else if vram_gb < 24.0 {
        90
    } else {
        100
    }
}

fn generate_recommendations(
    tier: DeviceTier,
    bottleneck: &Bottleneck,
    ram_gb: f64,
    vram_gb: f64,
) -> Vec<String> {
    let mut recs = Vec::new();

    match bottleneck {
        Bottleneck::Memory => {
            if ram_gb < 8.0 {
                recs.push("Consider upgrading to 16+ GB RAM for better model loading".to_string());
            }
            recs.push("Use smaller quantized models (Q3_K_M, Q4_K_M)".to_string());
            recs.push("Close unused applications to free memory".to_string());
        }
        Bottleneck::Gpu => {
            if vram_gb < 4.0 {
                recs.push("A GPU with 8+ GB VRAM would significantly improve inference speed".to_string());
            }
            recs.push("Use CPU-only models or smaller GPU-offloaded models".to_string());
        }
        Bottleneck::Cpu => {
            recs.push("Use GPU-accelerated inference when possible".to_string());
        }
        Bottleneck::Storage => {
            recs.push("An SSD would reduce model loading times by 5-10x".to_string());
        }
        Bottleneck::Thermal | Bottleneck::Balanced => {}
    }

    // Tier-specific recommendations
    match tier {
        DeviceTier::UltraLow => {
            recs.push("Using cloud providers is recommended for this hardware tier".to_string());
        }
        DeviceTier::Low => {
            recs.push("Local models will work but may be slow — cloud fallback is recommended".to_string());
        }
        _ => {}
    }

    recs
}
