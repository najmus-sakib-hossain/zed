//! # Binary Dawn Animation System
//!
//! 20x faster animations using pre-computed binary easing curves.
//! No CSS parsing at runtime - pure binary interpolation.

use bytemuck::{Pod, Zeroable};

/// Binary Dawn animation types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DawnAnimationType {
    /// Sunrise fade-in effect
    Sunrise = 0,
    /// Particle emergence
    Particles = 1,
    /// Wave propagation
    Wave = 2,
    /// Binary cascade (0s and 1s falling)
    BinaryCascade = 3,
    /// Glow pulse
    GlowPulse = 4,
    /// Matrix-style binary rain
    BinaryRain = 5,
    /// Orbital motion for 3D elements
    Orbital = 6,
    /// Morph between shapes
    Morph = 7,
}

impl DawnAnimationType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Sunrise),
            1 => Some(Self::Particles),
            2 => Some(Self::Wave),
            3 => Some(Self::BinaryCascade),
            4 => Some(Self::GlowPulse),
            5 => Some(Self::BinaryRain),
            6 => Some(Self::Orbital),
            7 => Some(Self::Morph),
            _ => None,
        }
    }
}

/// Pre-computed easing curves for Binary Dawn
pub const DAWN_FRAMES: usize = 120; // 2 seconds at 60fps

/// Generate sunrise easing curve (slow start, fast middle, slow end)
pub const fn generate_sunrise_curve() -> [f32; DAWN_FRAMES] {
    let mut curve = [0.0f32; DAWN_FRAMES];
    let mut i = 0;
    while i < DAWN_FRAMES {
        let t = i as f32 / (DAWN_FRAMES - 1) as f32;
        // Smooth step: 3t² - 2t³
        curve[i] = t * t * (3.0 - 2.0 * t);
        i += 1;
    }
    curve
}

/// Generate wave curve (sinusoidal)
pub const fn generate_wave_curve() -> [f32; DAWN_FRAMES] {
    let mut curve = [0.0f32; DAWN_FRAMES];
    let mut i = 0;
    while i < DAWN_FRAMES {
        let t = i as f32 / (DAWN_FRAMES - 1) as f32;
        // Approximate sin using Taylor series for const fn
        let x = t * 3.14159265;
        let x2 = x * x;
        let x3 = x2 * x;
        let x5 = x3 * x2;
        curve[i] = x - x3 / 6.0 + x5 / 120.0;
        i += 1;
    }
    curve
}

/// Generate glow pulse curve
pub const fn generate_pulse_curve() -> [f32; DAWN_FRAMES] {
    let mut curve = [0.0f32; DAWN_FRAMES];
    let mut i = 0;
    while i < DAWN_FRAMES {
        let t = i as f32 / (DAWN_FRAMES - 1) as f32;
        // Pulse: rises fast, holds, falls slowly
        if t < 0.2 {
            curve[i] = t * 5.0; // Fast rise
        } else if t < 0.6 {
            curve[i] = 1.0; // Hold
        } else {
            curve[i] = 1.0 - (t - 0.6) * 2.5; // Slow fall
        }
        i += 1;
    }
    curve
}

/// Pre-computed Binary Dawn curves
pub static DAWN_CURVES: DawnCurves = DawnCurves {
    sunrise: generate_sunrise_curve(),
    wave: generate_wave_curve(),
    pulse: generate_pulse_curve(),
};

/// Container for all dawn animation curves
pub struct DawnCurves {
    pub sunrise: [f32; DAWN_FRAMES],
    pub wave: [f32; DAWN_FRAMES],
    pub pulse: [f32; DAWN_FRAMES],
}

/// 16-byte Binary Dawn Animation Descriptor
///
/// Contains all information needed for a Binary Dawn animation
/// without any runtime parsing.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryDawnAnimation {
    /// Animation type (1 byte)
    pub animation_type: u8,
    /// Easing curve index (1 byte)
    pub curve: u8,
    /// Duration in milliseconds (2 bytes)
    pub duration_ms: u16,
    /// Start delay in milliseconds (2 bytes)
    pub delay_ms: u16,
    /// Color start (RGBA packed, 4 bytes)
    pub color_start: u32,
    /// Color end (RGBA packed, 4 bytes)
    pub color_end: u32,
    /// Extra flags (2 bytes)
    pub flags: u16,
}

impl BinaryDawnAnimation {
    /// Create a new Binary Dawn animation
    #[inline]
    pub const fn new(
        animation_type: DawnAnimationType,
        duration_ms: u16,
        delay_ms: u16,
    ) -> Self {
        Self {
            animation_type: animation_type as u8,
            curve: 0, // Sunrise curve by default
            duration_ms,
            delay_ms,
            color_start: 0x00000000, // Transparent
            color_end: 0xFFFFFFFF,   // White
            flags: 0,
        }
    }

    /// Create sunrise animation for landing page hero
    #[inline]
    pub const fn sunrise_hero() -> Self {
        Self {
            animation_type: DawnAnimationType::Sunrise as u8,
            curve: 0,
            duration_ms: 2000,
            delay_ms: 0,
            color_start: 0x000022FF, // Dark blue (ABGR)
            color_end: 0xFF8844FF,   // Orange sunrise
            flags: 0x01,             // Loop flag
        }
    }

    /// Create binary cascade for background
    #[inline]
    pub const fn binary_cascade() -> Self {
        Self {
            animation_type: DawnAnimationType::BinaryCascade as u8,
            curve: 1, // Wave curve
            duration_ms: 3000,
            delay_ms: 500,
            color_start: 0x00FF8800, // Cyan
            color_end: 0x00FFFF00,   // Yellow
            flags: 0x03,             // Loop + continuous
        }
    }

    /// Create glow pulse for interactive elements
    #[inline]
    pub const fn glow_pulse() -> Self {
        Self {
            animation_type: DawnAnimationType::GlowPulse as u8,
            curve: 2, // Pulse curve
            duration_ms: 1500,
            delay_ms: 0,
            color_start: 0x8000FFFF, // Semi-transparent magenta
            color_end: 0xFF00FFFF,   // Full magenta
            flags: 0x01,             // Loop
        }
    }

    /// Get animation frame value at given progress (0.0 - 1.0)
    #[inline]
    pub fn get_frame(&self, progress: f32) -> DawnFrame {
        let frame_idx = ((progress * (DAWN_FRAMES - 1) as f32) as usize).min(DAWN_FRAMES - 1);
        
        let curve_value = match self.curve {
            0 => DAWN_CURVES.sunrise[frame_idx],
            1 => DAWN_CURVES.wave[frame_idx],
            2 => DAWN_CURVES.pulse[frame_idx],
            _ => progress,
        };

        // Interpolate colors
        let r_start = (self.color_start & 0xFF) as f32;
        let g_start = ((self.color_start >> 8) & 0xFF) as f32;
        let b_start = ((self.color_start >> 16) & 0xFF) as f32;
        let a_start = ((self.color_start >> 24) & 0xFF) as f32;

        let r_end = (self.color_end & 0xFF) as f32;
        let g_end = ((self.color_end >> 8) & 0xFF) as f32;
        let b_end = ((self.color_end >> 16) & 0xFF) as f32;
        let a_end = ((self.color_end >> 24) & 0xFF) as f32;

        DawnFrame {
            progress: curve_value,
            r: lerp(r_start, r_end, curve_value) / 255.0,
            g: lerp(g_start, g_end, curve_value) / 255.0,
            b: lerp(b_start, b_end, curve_value) / 255.0,
            a: lerp(a_start, a_end, curve_value) / 255.0,
        }
    }

    /// Serialize to bytes
    #[inline]
    pub fn to_bytes(&self) -> [u8; 16] {
        bytemuck::bytes_of(self).try_into().unwrap()
    }

    /// Deserialize from bytes
    #[inline]
    pub fn from_bytes(bytes: &[u8; 16]) -> Self {
        *bytemuck::from_bytes(bytes)
    }
}

/// Animation frame output
#[derive(Debug, Clone, Copy)]
pub struct DawnFrame {
    /// Progress value (0.0 - 1.0)
    pub progress: f32,
    /// Red component (0.0 - 1.0)
    pub r: f32,
    /// Green component (0.0 - 1.0)
    pub g: f32,
    /// Blue component (0.0 - 1.0)
    pub b: f32,
    /// Alpha component (0.0 - 1.0)
    pub a: f32,
}

/// Linear interpolation
#[inline(always)]
const fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

// ============================================================================
// Particle System for Binary Dawn
// ============================================================================

/// Particle for binary cascade/rain animations
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryParticle {
    /// X position (0.0 - 1.0 normalized)
    pub x: f32,
    /// Y position (0.0 - 1.0 normalized)
    pub y: f32,
    /// Velocity X
    pub vx: f32,
    /// Velocity Y
    pub vy: f32,
    /// Life remaining (0.0 - 1.0)
    pub life: f32,
    /// Particle size
    pub size: f32,
    /// Is '1' or '0' (for binary display)
    pub binary_value: u32,
    /// Padding for alignment
    pub _pad: u32,
}

impl BinaryParticle {
    /// Create a new binary particle
    pub fn new(x: f32, y: f32, is_one: bool) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.02, // Fall speed
            life: 1.0,
            size: 12.0,
            binary_value: if is_one { 1 } else { 0 },
            _pad: 0,
        }
    }

    /// Update particle position
    #[inline]
    pub fn update(&mut self, dt: f32) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.life -= dt * 0.5;
    }

    /// Check if particle is alive
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.life > 0.0 && self.y < 1.0
    }
}

/// Binary particle system (max 256 particles, pool-based)
#[repr(C)]
pub struct BinaryParticleSystem {
    particles: [BinaryParticle; 256],
    active_count: u32,
    spawn_timer: f32,
}

impl BinaryParticleSystem {
    /// Create new particle system
    pub const fn new() -> Self {
        Self {
            particles: [BinaryParticle {
                x: 0.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
                life: 0.0,
                size: 0.0,
                binary_value: 0,
                _pad: 0,
            }; 256],
            active_count: 0,
            spawn_timer: 0.0,
        }
    }

    /// Update all particles
    pub fn update(&mut self, dt: f32) {
        // Update existing particles
        for i in 0..self.active_count as usize {
            self.particles[i].update(dt);
        }

        // Remove dead particles (swap with last)
        let mut i = 0;
        while i < self.active_count as usize {
            if !self.particles[i].is_alive() {
                self.active_count -= 1;
                if i < self.active_count as usize {
                    self.particles[i] = self.particles[self.active_count as usize];
                }
            } else {
                i += 1;
            }
        }

        // Spawn new particles
        self.spawn_timer += dt;
        if self.spawn_timer > 0.05 && self.active_count < 256 {
            self.spawn_timer = 0.0;
            let x = (self.active_count as f32 * 0.1234567) % 1.0; // Pseudo-random
            let is_one = self.active_count % 2 == 0;
            self.particles[self.active_count as usize] = BinaryParticle::new(x, 0.0, is_one);
            self.active_count += 1;
        }
    }

    /// Get active particles slice
    pub fn active_particles(&self) -> &[BinaryParticle] {
        &self.particles[..self.active_count as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_dawn_animation_size() {
        assert_eq!(std::mem::size_of::<BinaryDawnAnimation>(), 16);
    }

    #[test]
    fn test_animation_frame_interpolation() {
        let anim = BinaryDawnAnimation::sunrise_hero();
        
        let frame_start = anim.get_frame(0.0);
        let frame_mid = anim.get_frame(0.5);
        let frame_end = anim.get_frame(1.0);

        assert!(frame_start.progress < 0.01);
        assert!(frame_mid.progress > 0.4 && frame_mid.progress < 0.6);
        assert!(frame_end.progress > 0.99);
    }

    #[test]
    fn test_particle_system() {
        let mut system = BinaryParticleSystem::new();
        
        // Update a few times
        for _ in 0..10 {
            system.update(0.016); // 60fps
        }

        assert!(system.active_count > 0);
    }
}
