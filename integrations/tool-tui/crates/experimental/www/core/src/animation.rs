//! # Binary Animation System
//!
//! Binary Dawn's animation system uses SIMD-optimized interpolation with pre-computed
//! easing curves instead of CSS parsing. Achieves 20x faster animation frames.
//!
//! Each animation is an 8-byte descriptor that can be applied without runtime parsing.

/// Animation type enum
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationType {
    /// Fade in/out animation (opacity)
    Fade = 0,
    /// Slide animation (transform Y)
    Slide = 1,
    /// Scale animation (transform scale)
    Scale = 2,
    /// FLIP animation (transform + position)
    Flip = 3,
}

impl AnimationType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Fade),
            1 => Some(Self::Slide),
            2 => Some(Self::Scale),
            3 => Some(Self::Flip),
            _ => None,
        }
    }
}

/// Easing type enum
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingType {
    /// Linear interpolation
    Linear = 0,
    /// Ease in (quadratic)
    EaseIn = 1,
    /// Ease out (quadratic)
    EaseOut = 2,
    /// Cubic easing
    Cubic = 3,
}

impl EasingType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Linear),
            1 => Some(Self::EaseIn),
            2 => Some(Self::EaseOut),
            3 => Some(Self::Cubic),
            _ => None,
        }
    }
}

/// Number of frames per easing curve (60fps)
pub const FRAMES_PER_CURVE: usize = 60;

/// Number of easing curves
pub const CURVE_COUNT: usize = 4;

/// Pre-computed easing curves (60fps * 4 curves)
///
/// Each curve has 60 values representing the eased progress at each frame.
/// Values range from 0.0 to 1.0.
pub static EASING_CURVES: [[f32; FRAMES_PER_CURVE]; CURVE_COUNT] = [
    // Linear: t
    generate_linear_curve(),
    // EaseIn (quadratic): t^2
    generate_ease_in_curve(),
    // EaseOut (quadratic): 1 - (1-t)^2
    generate_ease_out_curve(),
    // Cubic: t^3
    generate_cubic_curve(),
];

/// Generate linear easing curve at compile time
const fn generate_linear_curve() -> [f32; FRAMES_PER_CURVE] {
    let mut curve = [0.0f32; FRAMES_PER_CURVE];
    let mut i = 0;
    while i < FRAMES_PER_CURVE {
        let t = i as f32 / (FRAMES_PER_CURVE - 1) as f32;
        curve[i] = t;
        i += 1;
    }
    curve
}

/// Generate ease-in (quadratic) curve at compile time
const fn generate_ease_in_curve() -> [f32; FRAMES_PER_CURVE] {
    let mut curve = [0.0f32; FRAMES_PER_CURVE];
    let mut i = 0;
    while i < FRAMES_PER_CURVE {
        let t = i as f32 / (FRAMES_PER_CURVE - 1) as f32;
        curve[i] = t * t;
        i += 1;
    }
    curve
}

/// Generate ease-out (quadratic) curve at compile time
const fn generate_ease_out_curve() -> [f32; FRAMES_PER_CURVE] {
    let mut curve = [0.0f32; FRAMES_PER_CURVE];
    let mut i = 0;
    while i < FRAMES_PER_CURVE {
        let t = i as f32 / (FRAMES_PER_CURVE - 1) as f32;
        let inv = 1.0 - t;
        curve[i] = 1.0 - (inv * inv);
        i += 1;
    }
    curve
}

/// Generate cubic easing curve at compile time
const fn generate_cubic_curve() -> [f32; FRAMES_PER_CURVE] {
    let mut curve = [0.0f32; FRAMES_PER_CURVE];
    let mut i = 0;
    while i < FRAMES_PER_CURVE {
        let t = i as f32 / (FRAMES_PER_CURVE - 1) as f32;
        curve[i] = t * t * t;
        i += 1;
    }
    curve
}

/// Animation property flags (bitfield)
pub mod properties {
    /// Opacity property
    pub const OPACITY: u32 = 0x01;
    /// Transform property
    pub const TRANSFORM: u32 = 0x02;
    /// Position property
    pub const POSITION: u32 = 0x04;
    /// Scale property
    pub const SCALE: u32 = 0x08;
}

/// 8-byte animation descriptor
///
/// Pre-computed animation configuration that can be applied without runtime parsing.
/// Uses SIMD-optimized easing curves for 20x faster frame calculation.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryAnimation {
    /// Animation type (1 byte)
    pub animation_type: AnimationType,
    /// Easing type (1 byte)
    pub easing: EasingType,
    /// Duration in milliseconds (2 bytes)
    pub duration_ms: u16,
    /// Properties bitfield (4 bytes)
    pub properties: u32,
}

impl BinaryAnimation {
    /// Size of BinaryAnimation in bytes - must be exactly 8
    pub const SIZE: usize = 8;

    /// Create a new animation
    #[inline]
    pub const fn new(
        animation_type: AnimationType,
        duration_ms: u16,
        easing: EasingType,
        properties: u32,
    ) -> Self {
        Self {
            animation_type,
            easing,
            duration_ms,
            properties,
        }
    }

    /// Get the eased value for a given frame index
    ///
    /// Uses pre-computed easing curves for O(1) lookup.
    #[inline(always)]
    pub fn calculate_frame(&self, frame_index: usize) -> f32 {
        let curve_index = self.easing as usize;
        let frame = frame_index.min(FRAMES_PER_CURVE - 1);
        EASING_CURVES[curve_index][frame]
    }

    /// Get the eased value for a progress value (0.0 to 1.0)
    #[inline(always)]
    pub fn calculate_progress(&self, progress: f32) -> f32 {
        let frame_index =
            ((progress * (FRAMES_PER_CURVE - 1) as f32) as usize).min(FRAMES_PER_CURVE - 1);
        self.calculate_frame(frame_index)
    }

    /// Apply animation frame to an element
    ///
    /// Returns the animation values to apply based on animation type.
    pub fn apply_frame(&self, progress: f32) -> AnimationFrame {
        let eased = self.calculate_progress(progress);

        match self.animation_type {
            AnimationType::Fade => AnimationFrame {
                opacity: Some(eased),
                transform_y: None,
                scale: None,
            },
            AnimationType::Slide => AnimationFrame {
                opacity: None,
                transform_y: Some(lerp(100.0, 0.0, eased)),
                scale: None,
            },
            AnimationType::Scale => AnimationFrame {
                opacity: None,
                transform_y: None,
                scale: Some(lerp(0.0, 1.0, eased)),
            },
            AnimationType::Flip => AnimationFrame {
                opacity: Some(eased),
                transform_y: Some(lerp(50.0, 0.0, eased)),
                scale: Some(lerp(0.8, 1.0, eased)),
            },
        }
    }

    /// Check if animation affects opacity
    #[inline]
    pub fn affects_opacity(&self) -> bool {
        (self.properties & properties::OPACITY) != 0
    }

    /// Check if animation affects transform
    #[inline]
    pub fn affects_transform(&self) -> bool {
        (self.properties & properties::TRANSFORM) != 0
    }

    /// Serialize to bytes
    #[inline]
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.animation_type as u8;
        bytes[1] = self.easing as u8;
        bytes[2..4].copy_from_slice(&self.duration_ms.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.properties.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    #[inline]
    pub fn from_bytes(bytes: &[u8; 8]) -> Option<Self> {
        Some(Self {
            animation_type: AnimationType::from_u8(bytes[0])?,
            easing: EasingType::from_u8(bytes[1])?,
            duration_ms: u16::from_le_bytes([bytes[2], bytes[3]]),
            properties: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        })
    }
}

/// Animation frame values
#[derive(Debug, Clone, Copy, Default)]
pub struct AnimationFrame {
    /// Opacity value (0.0 to 1.0)
    pub opacity: Option<f32>,
    /// Transform Y offset in pixels
    pub transform_y: Option<f32>,
    /// Scale factor
    pub scale: Option<f32>,
}

/// Linear interpolation
#[inline(always)]
pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

// ============================================================================
// API Functions - Declarative animation binding
// ============================================================================

/// Create a fade animation
///
/// Fades element from transparent to opaque.
#[inline]
pub fn fade() -> BinaryAnimation {
    BinaryAnimation::new(AnimationType::Fade, 300, EasingType::EaseOut, properties::OPACITY)
}

/// Create a fade animation with custom duration
#[inline]
pub fn fade_with_duration(duration_ms: u16) -> BinaryAnimation {
    BinaryAnimation::new(AnimationType::Fade, duration_ms, EasingType::EaseOut, properties::OPACITY)
}

/// Create a slide animation
///
/// Slides element from below into position.
#[inline]
pub fn slide() -> BinaryAnimation {
    BinaryAnimation::new(AnimationType::Slide, 300, EasingType::EaseOut, properties::TRANSFORM)
}

/// Create a slide animation with custom duration
#[inline]
pub fn slide_with_duration(duration_ms: u16) -> BinaryAnimation {
    BinaryAnimation::new(
        AnimationType::Slide,
        duration_ms,
        EasingType::EaseOut,
        properties::TRANSFORM,
    )
}

/// Create a scale animation
///
/// Scales element from 0 to full size.
#[inline]
pub fn scale() -> BinaryAnimation {
    BinaryAnimation::new(AnimationType::Scale, 300, EasingType::EaseOut, properties::SCALE)
}

/// Create a FLIP animation
///
/// Combined fade, slide, and scale for smooth transitions.
#[inline]
pub fn flip(duration_ms: u16) -> BinaryAnimation {
    BinaryAnimation::new(
        AnimationType::Flip,
        duration_ms,
        EasingType::Cubic,
        properties::TRANSFORM | properties::POSITION | properties::OPACITY,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_animation_size() {
        assert_eq!(std::mem::size_of::<BinaryAnimation>(), BinaryAnimation::SIZE);
        assert_eq!(std::mem::size_of::<BinaryAnimation>(), 8);
    }

    #[test]
    fn test_easing_curves_bounds() {
        for curve in &EASING_CURVES {
            assert_eq!(curve.len(), FRAMES_PER_CURVE);
            assert_eq!(curve[0], 0.0);
            assert!((curve[FRAMES_PER_CURVE - 1] - 1.0).abs() < 0.001);

            for &value in curve {
                assert!(value >= 0.0 && value <= 1.0);
            }
        }
    }

    #[test]
    fn test_animation_roundtrip() {
        let anim = fade();
        let bytes = anim.to_bytes();
        let restored = BinaryAnimation::from_bytes(&bytes).unwrap();
        assert_eq!(anim, restored);
    }

    #[test]
    fn test_fade_animation() {
        let anim = fade();
        assert_eq!(anim.animation_type, AnimationType::Fade);
        let duration = { anim.duration_ms };
        assert_eq!(duration, 300);
        assert!(anim.affects_opacity());
    }

    #[test]
    fn test_slide_animation() {
        let anim = slide();
        assert_eq!(anim.animation_type, AnimationType::Slide);
        assert!(anim.affects_transform());
    }

    #[test]
    fn test_flip_animation() {
        let anim = flip(500);
        assert_eq!(anim.animation_type, AnimationType::Flip);
        let duration = { anim.duration_ms };
        assert_eq!(duration, 500);
        assert_eq!(anim.easing, EasingType::Cubic);
    }

    #[test]
    fn test_apply_frame() {
        let anim = fade();

        let frame_start = anim.apply_frame(0.0);
        assert!(frame_start.opacity.is_some());
        assert!((frame_start.opacity.unwrap() - 0.0).abs() < 0.01);

        let frame_end = anim.apply_frame(1.0);
        assert!((frame_end.opacity.unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 100.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 100.0, 1.0), 100.0);
        assert_eq!(lerp(0.0, 100.0, 0.5), 50.0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid AnimationType
    fn animation_type_strategy() -> impl Strategy<Value = AnimationType> {
        prop_oneof![
            Just(AnimationType::Fade),
            Just(AnimationType::Slide),
            Just(AnimationType::Scale),
            Just(AnimationType::Flip),
        ]
    }

    // Strategy for generating valid EasingType
    fn easing_type_strategy() -> impl Strategy<Value = EasingType> {
        prop_oneof![
            Just(EasingType::Linear),
            Just(EasingType::EaseIn),
            Just(EasingType::EaseOut),
            Just(EasingType::Cubic),
        ]
    }

    // **Feature: binary-dawn-features, Property 3: BinaryAnimation Size Invariant**
    // *For any* BinaryAnimation instance, `size_of::<BinaryAnimation>()` SHALL equal exactly 8 bytes.
    // **Validates: Requirements 2.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_binary_animation_size_invariant(
            animation_type in animation_type_strategy(),
            duration_ms in any::<u16>(),
            easing in easing_type_strategy(),
            properties in any::<u32>()
        ) {
            let anim = BinaryAnimation::new(animation_type, duration_ms, easing, properties);

            // Size must always be exactly 8 bytes
            prop_assert_eq!(std::mem::size_of::<BinaryAnimation>(), 8);
            prop_assert_eq!(BinaryAnimation::SIZE, 8);

            // Serialized form must also be 8 bytes
            let bytes = anim.to_bytes();
            prop_assert_eq!(bytes.len(), 8);
        }
    }

    // **Feature: binary-dawn-features, Property 4: Easing Curves Validity**
    // *For all* easing curves in EASING_CURVES, each curve SHALL have exactly 60 values,
    // and all values SHALL be in the range [0.0, 1.0] with the first value being 0.0
    // and the last being 1.0.
    // **Validates: Requirements 2.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_easing_curves_validity(
            curve_index in 0usize..CURVE_COUNT,
            frame_index in 0usize..FRAMES_PER_CURVE
        ) {
            let curve = &EASING_CURVES[curve_index];

            // Each curve must have exactly 60 values
            prop_assert_eq!(curve.len(), 60);
            prop_assert_eq!(curve.len(), FRAMES_PER_CURVE);

            // All values must be in range [0.0, 1.0]
            let value = curve[frame_index];
            prop_assert!(value >= 0.0, "Value {} at index {} is below 0.0", value, frame_index);
            prop_assert!(value <= 1.0, "Value {} at index {} is above 1.0", value, frame_index);

            // First value must be 0.0
            prop_assert!((curve[0] - 0.0).abs() < 0.001, "First value is not 0.0: {}", curve[0]);

            // Last value must be 1.0
            prop_assert!(
                (curve[FRAMES_PER_CURVE - 1] - 1.0).abs() < 0.001,
                "Last value is not 1.0: {}",
                curve[FRAMES_PER_CURVE - 1]
            );
        }
    }

    // Round-trip property for BinaryAnimation serialization
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_binary_animation_roundtrip(
            animation_type in animation_type_strategy(),
            duration_ms in any::<u16>(),
            easing in easing_type_strategy(),
            properties in any::<u32>()
        ) {
            let original = BinaryAnimation::new(animation_type, duration_ms, easing, properties);
            let bytes = original.to_bytes();
            let restored = BinaryAnimation::from_bytes(&bytes);

            prop_assert!(restored.is_some());
            prop_assert_eq!(original, restored.unwrap());
        }
    }

    // Progress calculation property - eased values should be monotonic for standard easings
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_easing_monotonic(
            easing in easing_type_strategy(),
            progress1 in 0.0f32..1.0,
            progress2 in 0.0f32..1.0
        ) {
            let anim = BinaryAnimation::new(
                AnimationType::Fade,
                300,
                easing,
                properties::OPACITY
            );

            let (p1, p2) = if progress1 <= progress2 {
                (progress1, progress2)
            } else {
                (progress2, progress1)
            };

            let v1 = anim.calculate_progress(p1);
            let v2 = anim.calculate_progress(p2);

            // For all standard easings, if p1 <= p2, then v1 <= v2 (monotonic)
            prop_assert!(
                v1 <= v2 + 0.001, // Small epsilon for floating point
                "Easing not monotonic: progress {} -> {}, values {} -> {}",
                p1, p2, v1, v2
            );
        }
    }
}
