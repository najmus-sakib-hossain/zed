//! SIMD operations for vectorized computation
//!
//! This module provides SIMD-accelerated operations for array processing.
//! It uses architecture-specific intrinsics when available (AVX2, SSE2, NEON)
//! and falls back to scalar operations on unsupported platforms.

pub mod console;

pub use console::{console_flush, console_log_number, BatchConsole};

use crate::error::{DxError, DxResult};

/// Check if AVX2 is available at runtime
#[cfg(target_arch = "x86_64")]
fn has_avx2() -> bool {
    is_x86_feature_detected!("avx2")
}

#[cfg(not(target_arch = "x86_64"))]
fn has_avx2() -> bool {
    false
}

/// Check if SSE2 is available at runtime
#[cfg(target_arch = "x86_64")]
fn has_sse2() -> bool {
    is_x86_feature_detected!("sse2")
}

#[cfg(not(target_arch = "x86_64"))]
fn has_sse2() -> bool {
    false
}

#[derive(Debug, Clone)]
pub struct SimdF32x4(pub [f32; 4]);

#[derive(Debug, Clone)]
pub struct SimdF32x8(pub [f32; 8]);

#[derive(Debug, Clone)]
pub struct SimdF64x2(pub [f64; 2]);

#[derive(Debug, Clone)]
pub struct SimdF64x4(pub [f64; 4]);

#[derive(Debug, Clone)]
pub struct SimdI32x4(pub [i32; 4]);

#[derive(Debug, Clone)]
pub struct SimdI32x8(pub [i32; 8]);

impl SimdF32x4 {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        Self([a, b, c, d])
    }

    pub fn splat(value: f32) -> Self {
        Self([value; 4])
    }

    pub fn load(slice: &[f32]) -> Self {
        debug_assert!(slice.len() >= 4);
        Self([slice[0], slice[1], slice[2], slice[3]])
    }

    pub fn store(&self, slice: &mut [f32]) {
        debug_assert!(slice.len() >= 4);
        slice[..4].copy_from_slice(&self.0);
    }

    pub fn add(&self, other: &Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
            self.0[3] + other.0[3],
        ])
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self([
            self.0[0] * other.0[0],
            self.0[1] * other.0[1],
            self.0[2] * other.0[2],
            self.0[3] * other.0[3],
        ])
    }

    pub fn sum(&self) -> f32 {
        self.0.iter().sum()
    }
}

impl SimdF32x8 {
    pub fn splat(value: f32) -> Self {
        Self([value; 8])
    }

    pub fn load(slice: &[f32]) -> Self {
        debug_assert!(slice.len() >= 8);
        Self([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ])
    }

    pub fn store(&self, slice: &mut [f32]) {
        debug_assert!(slice.len() >= 8);
        slice[..8].copy_from_slice(&self.0);
    }

    pub fn add(&self, other: &Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
            self.0[3] + other.0[3],
            self.0[4] + other.0[4],
            self.0[5] + other.0[5],
            self.0[6] + other.0[6],
            self.0[7] + other.0[7],
        ])
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self([
            self.0[0] * other.0[0],
            self.0[1] * other.0[1],
            self.0[2] * other.0[2],
            self.0[3] * other.0[3],
            self.0[4] * other.0[4],
            self.0[5] * other.0[5],
            self.0[6] * other.0[6],
            self.0[7] * other.0[7],
        ])
    }

    pub fn sum(&self) -> f32 {
        self.0.iter().sum()
    }
}

impl SimdF64x2 {
    pub fn splat(value: f64) -> Self {
        Self([value; 2])
    }

    pub fn load(slice: &[f64]) -> Self {
        debug_assert!(slice.len() >= 2);
        Self([slice[0], slice[1]])
    }

    pub fn store(&self, slice: &mut [f64]) {
        debug_assert!(slice.len() >= 2);
        slice[..2].copy_from_slice(&self.0);
    }

    pub fn add(&self, other: &Self) -> Self {
        Self([self.0[0] + other.0[0], self.0[1] + other.0[1]])
    }

    pub fn sum(&self) -> f64 {
        self.0[0] + self.0[1]
    }
}

impl SimdF64x4 {
    pub fn splat(value: f64) -> Self {
        Self([value; 4])
    }

    pub fn load(slice: &[f64]) -> Self {
        debug_assert!(slice.len() >= 4);
        Self([slice[0], slice[1], slice[2], slice[3]])
    }

    pub fn store(&self, slice: &mut [f64]) {
        debug_assert!(slice.len() >= 4);
        slice[..4].copy_from_slice(&self.0);
    }

    pub fn add(&self, other: &Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
            self.0[3] + other.0[3],
        ])
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self([
            self.0[0] * other.0[0],
            self.0[1] * other.0[1],
            self.0[2] * other.0[2],
            self.0[3] * other.0[3],
        ])
    }

    pub fn sum(&self) -> f64 {
        self.0.iter().sum()
    }
}

impl SimdI32x4 {
    pub fn new(a: i32, b: i32, c: i32, d: i32) -> Self {
        Self([a, b, c, d])
    }

    pub fn splat(value: i32) -> Self {
        Self([value; 4])
    }

    pub fn add(&self, other: &Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
            self.0[3] + other.0[3],
        ])
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self([
            self.0[0] * other.0[0],
            self.0[1] * other.0[1],
            self.0[2] * other.0[2],
            self.0[3] * other.0[3],
        ])
    }
}

impl SimdI32x8 {
    pub fn splat(value: i32) -> Self {
        Self([value; 8])
    }

    pub fn load(slice: &[i32]) -> Self {
        debug_assert!(slice.len() >= 8);
        Self([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ])
    }

    pub fn store(&self, slice: &mut [i32]) {
        debug_assert!(slice.len() >= 8);
        slice[..8].copy_from_slice(&self.0);
    }

    pub fn add(&self, other: &Self) -> Self {
        Self([
            self.0[0].wrapping_add(other.0[0]),
            self.0[1].wrapping_add(other.0[1]),
            self.0[2].wrapping_add(other.0[2]),
            self.0[3].wrapping_add(other.0[3]),
            self.0[4].wrapping_add(other.0[4]),
            self.0[5].wrapping_add(other.0[5]),
            self.0[6].wrapping_add(other.0[6]),
            self.0[7].wrapping_add(other.0[7]),
        ])
    }
}

/// Vectorized array addition for f32
/// Uses AVX2 (8-wide) if available, falls back to SSE2 (4-wide) or scalar
pub fn vector_add_f32(a: &[f32], b: &[f32], result: &mut [f32]) -> DxResult<()> {
    if a.len() != b.len() || a.len() != result.len() {
        return Err(DxError::RuntimeError("Vector length mismatch".to_string()));
    }

    if has_avx2() && a.len() >= 8 {
        vector_add_f32_avx2(a, b, result)
    } else if has_sse2() && a.len() >= 4 {
        vector_add_f32_sse2(a, b, result)
    } else {
        vector_add_f32_scalar(a, b, result)
    }
}

/// AVX2 implementation (8-wide f32)
fn vector_add_f32_avx2(a: &[f32], b: &[f32], result: &mut [f32]) -> DxResult<()> {
    let chunks = a.len() / 8;
    for i in 0..chunks {
        let offset = i * 8;
        let va = SimdF32x8::load(&a[offset..]);
        let vb = SimdF32x8::load(&b[offset..]);
        let vr = va.add(&vb);
        vr.store(&mut result[offset..]);
    }

    // Handle remainder with scalar
    for i in (chunks * 8)..a.len() {
        result[i] = a[i] + b[i];
    }

    Ok(())
}

/// SSE2 implementation (4-wide f32)
fn vector_add_f32_sse2(a: &[f32], b: &[f32], result: &mut [f32]) -> DxResult<()> {
    let chunks = a.len() / 4;
    for i in 0..chunks {
        let offset = i * 4;
        let va = SimdF32x4::load(&a[offset..]);
        let vb = SimdF32x4::load(&b[offset..]);
        let vr = va.add(&vb);
        vr.store(&mut result[offset..]);
    }

    // Handle remainder
    for i in (chunks * 4)..a.len() {
        result[i] = a[i] + b[i];
    }

    Ok(())
}

/// Scalar fallback implementation
fn vector_add_f32_scalar(a: &[f32], b: &[f32], result: &mut [f32]) -> DxResult<()> {
    for i in 0..a.len() {
        result[i] = a[i] + b[i];
    }
    Ok(())
}

/// Vectorized array addition for f64
pub fn vector_add_f64(a: &[f64], b: &[f64], result: &mut [f64]) -> DxResult<()> {
    if a.len() != b.len() || a.len() != result.len() {
        return Err(DxError::RuntimeError("Vector length mismatch".to_string()));
    }

    if has_avx2() && a.len() >= 4 {
        let chunks = a.len() / 4;
        for i in 0..chunks {
            let offset = i * 4;
            let va = SimdF64x4::load(&a[offset..]);
            let vb = SimdF64x4::load(&b[offset..]);
            let vr = va.add(&vb);
            vr.store(&mut result[offset..]);
        }
        for i in (chunks * 4)..a.len() {
            result[i] = a[i] + b[i];
        }
    } else if has_sse2() && a.len() >= 2 {
        let chunks = a.len() / 2;
        for i in 0..chunks {
            let offset = i * 2;
            let va = SimdF64x2::load(&a[offset..]);
            let vb = SimdF64x2::load(&b[offset..]);
            let vr = va.add(&vb);
            vr.store(&mut result[offset..]);
        }
        for i in (chunks * 2)..a.len() {
            result[i] = a[i] + b[i];
        }
    } else {
        for i in 0..a.len() {
            result[i] = a[i] + b[i];
        }
    }

    Ok(())
}

/// Vectorized dot product for f32
pub fn vector_dot_f32(a: &[f32], b: &[f32]) -> DxResult<f32> {
    if a.len() != b.len() {
        return Err(DxError::RuntimeError("Vector length mismatch".to_string()));
    }

    let mut sum = 0.0f32;

    if has_avx2() && a.len() >= 8 {
        let chunks = a.len() / 8;
        let mut acc = SimdF32x8::splat(0.0);
        for i in 0..chunks {
            let offset = i * 8;
            let va = SimdF32x8::load(&a[offset..]);
            let vb = SimdF32x8::load(&b[offset..]);
            let prod = va.mul(&vb);
            acc = acc.add(&prod);
        }
        sum = acc.sum();
        for i in (chunks * 8)..a.len() {
            sum += a[i] * b[i];
        }
    } else if has_sse2() && a.len() >= 4 {
        let chunks = a.len() / 4;
        let mut acc = SimdF32x4::splat(0.0);
        for i in 0..chunks {
            let offset = i * 4;
            let va = SimdF32x4::load(&a[offset..]);
            let vb = SimdF32x4::load(&b[offset..]);
            let prod = va.mul(&vb);
            acc = acc.add(&prod);
        }
        sum = acc.sum();
        for i in (chunks * 4)..a.len() {
            sum += a[i] * b[i];
        }
    } else {
        for i in 0..a.len() {
            sum += a[i] * b[i];
        }
    }

    Ok(sum)
}

/// Vectorized sum for f64 (JavaScript numbers)
pub fn vector_sum_f64(a: &[f64]) -> f64 {
    if has_avx2() && a.len() >= 4 {
        let chunks = a.len() / 4;
        let mut acc = SimdF64x4::splat(0.0);
        for i in 0..chunks {
            let offset = i * 4;
            let va = SimdF64x4::load(&a[offset..]);
            acc = acc.add(&va);
        }
        let mut sum = acc.sum();
        for val in a.iter().skip(chunks * 4) {
            sum += val;
        }
        sum
    } else if has_sse2() && a.len() >= 2 {
        let chunks = a.len() / 2;
        let mut acc = SimdF64x2::splat(0.0);
        for i in 0..chunks {
            let offset = i * 2;
            let va = SimdF64x2::load(&a[offset..]);
            acc = acc.add(&va);
        }
        let mut sum = acc.sum();
        for val in a.iter().skip(chunks * 2) {
            sum += val;
        }
        sum
    } else {
        a.iter().sum()
    }
}
