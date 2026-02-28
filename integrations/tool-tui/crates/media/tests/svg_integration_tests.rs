//! Integration tests for SVG processing.

mod common;

use common::TestFixture;

#[cfg(feature = "image-svg")]
#[test]
fn test_svg_to_png_conversion() {
    use dx_media::tools::image::svg::svg_to_png;

    let fixture = TestFixture::new();

    // Create a simple SVG
    let svg_content = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
        <rect width="100" height="100" fill="blue"/>
    </svg>"#;

    let input = fixture.create_test_text_file("test.svg", svg_content);
    let output = fixture.path("test.png");

    let result = svg_to_png(&input, &output, 100, 100);
    assert!(result.is_ok(), "SVG to PNG conversion should succeed: {:?}", result.err());
    assert!(output.exists(), "Output PNG should exist");

    let metadata = std::fs::metadata(&output).unwrap();
    assert!(metadata.len() > 0, "PNG file should not be empty");
}

#[cfg(feature = "image-svg")]
#[test]
fn test_svg_to_png_with_aspect_ratio() {
    use dx_media::tools::image::svg::svg_to_png_width;

    let fixture = TestFixture::new();

    let svg_content = r#"<svg width="200" height="100" xmlns="http://www.w3.org/2000/svg">
        <rect width="200" height="100" fill="red"/>
    </svg>"#;

    let input = fixture.create_test_text_file("test.svg", svg_content);
    let output = fixture.path("test.png");

    let result = svg_to_png_width(&input, &output, 100);
    assert!(
        result.is_ok(),
        "SVG to PNG with aspect ratio should succeed: {:?}",
        result.err()
    );
    assert!(output.exists(), "Output PNG should exist");
}

#[cfg(feature = "image-svg")]
#[test]
fn test_generate_web_icons() {
    use dx_media::tools::image::svg::generate_web_icons;

    let fixture = TestFixture::new();

    let svg_content = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
        <circle cx="50" cy="50" r="40" fill="green"/>
    </svg>"#;

    let input = fixture.create_test_text_file("icon.svg", svg_content);
    let output_dir = fixture.path("icons");

    let result = generate_web_icons(&input, &output_dir);
    assert!(result.is_ok(), "Web icon generation should succeed: {:?}", result.err());
    assert!(output_dir.exists(), "Output directory should exist");

    // Check that multiple sizes were generated
    let result = result.unwrap();
    assert!(result.output_paths.len() >= 5, "Should generate multiple icon sizes");
}

#[cfg(not(feature = "image-svg"))]
#[test]
fn test_svg_feature_disabled() {
    assert!(true, "SVG feature is disabled");
}
