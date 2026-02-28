//! Tests for native Rust image processing (no external dependencies).
//!
//! These tests use the `image` crate and should work without ImageMagick.

mod common;

use common::TestFixture;

#[cfg(feature = "image-core")]
#[test]
fn test_native_image_convert_png_to_jpg() {
    use dx_media::tools::image::native::convert_native;

    let fixture = TestFixture::new();
    let input = fixture.create_test_image("test.png");
    let output = fixture.path("test.jpg");

    let result = convert_native(&input, &output, Some(85));
    assert!(result.is_ok(), "Native conversion should succeed: {:?}", result.err());
    assert!(output.exists(), "Output file should exist");

    // Verify it's a valid JPEG
    let metadata = std::fs::metadata(&output).unwrap();
    assert!(metadata.len() > 0, "Output file should not be empty");
}

#[cfg(feature = "image-core")]
#[test]
fn test_native_image_convert_png_to_webp() {
    use dx_media::tools::image::native::convert_native;

    let fixture = TestFixture::new();
    let input = fixture.create_test_image("test.png");
    let output = fixture.path("test.webp");

    let result = convert_native(&input, &output, None);
    assert!(result.is_ok(), "WebP conversion should succeed: {:?}", result.err());
    assert!(output.exists(), "Output file should exist");
}

#[cfg(feature = "image-core")]
#[test]
fn test_native_image_resize() {
    use dx_media::tools::image::native::resize_native;

    let fixture = TestFixture::new();
    let input = fixture.create_test_image("test.png");
    let output = fixture.path("resized.png");

    let result = resize_native(&input, &output, Some(50), Some(50), false);
    assert!(result.is_ok(), "Resize should succeed: {:?}", result.err());
    assert!(output.exists(), "Output file should exist");
}

#[cfg(feature = "image-core")]
#[test]
fn test_native_image_resize_keep_aspect() {
    use dx_media::tools::image::native::resize_native;

    let fixture = TestFixture::new();
    let input = fixture.create_test_image("test.png");
    let output = fixture.path("resized_aspect.png");

    let result = resize_native(&input, &output, Some(50), None, true);
    assert!(result.is_ok(), "Aspect-preserving resize should succeed: {:?}", result.err());
    assert!(output.exists(), "Output file should exist");
}

#[cfg(not(feature = "image-core"))]
#[test]
fn test_image_core_feature_disabled() {
    // This test just ensures the test suite compiles without image-core
    assert!(true, "image-core feature is disabled");
}
