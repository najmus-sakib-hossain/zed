//! Shared utility: Convert any RGBA buffer into a GPUI-displayable image.
//!
//! This is the **crucial bridge** between every decoder/renderer and GPUI's display.
//! Core pattern: decode/render content → RGBA bitmap → display via GPUI `img()` element.

use gpui::*;
use image::{DynamicImage, Frame, RgbaImage};
use std::sync::Arc;

/// Wraps an RGBA bitmap so GPUI can display it with `img()`.
///
/// GPUI's `img()` can take an `ImageSource`, which can be created from
/// a `RenderImage` (an in-memory RGBA pixel buffer).
pub fn rgba_to_gpui_image(rgba: &RgbaImage) -> ImageSource {
    let width = rgba.width();
    let height = rgba.height();
    let pixels = rgba.as_raw().clone();

    // GPUI's RenderImage expects frames (for animated images)
    // For a single frame, just provide one
    let rgba_image = image::RgbaImage::from_raw(width, height, pixels)
        .expect("Invalid RGBA buffer");
    let frame = Frame::new(rgba_image);

    let render_image = RenderImage::new(vec![frame]);
    ImageSource::Render(Arc::new(render_image))
}

/// Convenience: load a DynamicImage and convert
#[allow(dead_code)]
pub fn dynamic_image_to_gpui(dyn_img: &DynamicImage) -> ImageSource {
    let rgba = dyn_img.to_rgba8();
    rgba_to_gpui_image(&rgba)
}

/// Creates a GPUI-displayable element from raw pixel data
#[allow(dead_code)]
pub fn bitmap_element(
    source: ImageSource,
    width: f32,
    height: f32,
) -> impl IntoElement {
    img(source)
        .w(px(width))
        .h(px(height))
        .object_fit(ObjectFit::Contain)
}
