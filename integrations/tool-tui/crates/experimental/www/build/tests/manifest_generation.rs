//! Property tests for manifest generation
//!
//! Feature: dx-www-production-ready, Property 2: Manifest Icon References
//!
//! This property test verifies that for any generated favicon set, the manifest.json
//! contains valid references to all generated icon files. The property tests that:
//! 1. All generated icons are referenced in the manifest
//! 2. Each reference has correct src, sizes, and type fields
//! 3. The manifest JSON is valid and parseable
//! 4. All referenced files actually exist

use build::{BuildCache, FaviconSize, MediaConfig, MediaProcessor};
use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Strategy for generating arbitrary image content
fn arbitrary_image_content() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 100..2048)
}

/// Strategy for generating a subset of favicon sizes
fn arbitrary_favicon_sizes() -> impl Strategy<Value = Vec<FaviconSize>> {
    prop::collection::vec(
        prop_oneof![
            Just(FaviconSize::Ico16),
            Just(FaviconSize::Ico32),
            Just(FaviconSize::Ico48),
            Just(FaviconSize::Png16),
            Just(FaviconSize::Png32),
            Just(FaviconSize::AppleTouch180),
            Just(FaviconSize::AndroidChrome192),
            Just(FaviconSize::AndroidChrome512),
        ],
        1..=8,
    )
    .prop_map(|mut sizes| {
        // Remove duplicates
        sizes.sort_by_key(|s| format!("{:?}", s));
        sizes.dedup();
        sizes
    })
}

proptest! {
    /// Property 2a: All generated icons are referenced in manifest
    ///
    /// For any generated favicon set, every icon file that exists on disk
    /// should have a corresponding entry in the manifest.json.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn all_icons_referenced_in_manifest(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: sizes.clone(),
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Generate manifest JSON
        let json_result = processor.generate_manifest_json(&manifest);
        prop_assert!(json_result.is_ok(), "Manifest JSON generation should succeed");

        let json_str = json_result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Verify all generated files are referenced in manifest
        for size in &sizes {
            let filename = size.filename();
            let file_path = output_dir.join(filename);

            // File should exist
            prop_assert!(
                file_path.exists(),
                "Generated file should exist: {}",
                filename
            );

            // File should be referenced in manifest JSON
            let icons = json["icons"].as_array().unwrap();
            let found = icons.iter().any(|icon| {
                icon["src"].as_str() == Some(filename)
            });

            prop_assert!(
                found,
                "Manifest should reference generated file: {}",
                filename
            );
        }
    }

    /// Property 2b: Each manifest reference has correct src field
    ///
    /// For any generated favicon set, each icon entry in the manifest.json
    /// should have a valid 'src' field that matches the actual filename.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn manifest_references_have_correct_src(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: sizes.clone(),
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Generate manifest JSON
        let json_result = processor.generate_manifest_json(&manifest);
        prop_assert!(json_result.is_ok(), "Manifest JSON generation should succeed");

        let json_str = json_result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let icons = json["icons"].as_array().unwrap();

        // Verify each icon entry has a valid src field
        for icon in icons {
            let src = icon["src"].as_str();
            prop_assert!(
                src.is_some(),
                "Icon entry should have 'src' field"
            );

            let src = src.unwrap();
            prop_assert!(
                !src.is_empty(),
                "Icon 'src' field should not be empty"
            );

            // Verify src matches one of the expected filenames
            let valid_filename = sizes.iter().any(|size| size.filename() == src);
            prop_assert!(
                valid_filename,
                "Icon src '{}' should match a configured size filename",
                src
            );
        }
    }

    /// Property 2c: Each manifest reference has correct sizes field
    ///
    /// For any generated favicon set, each icon entry in the manifest.json
    /// should have a 'sizes' field in "WxH" format matching the actual dimensions.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn manifest_references_have_correct_sizes(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: sizes.clone(),
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Generate manifest JSON
        let json_result = processor.generate_manifest_json(&manifest);
        prop_assert!(json_result.is_ok(), "Manifest JSON generation should succeed");

        let json_str = json_result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let icons = json["icons"].as_array().unwrap();

        // Verify each icon entry has correct sizes field
        for (size, icon) in sizes.iter().zip(icons.iter()) {
            let sizes_field = icon["sizes"].as_str();
            prop_assert!(
                sizes_field.is_some(),
                "Icon entry should have 'sizes' field"
            );

            let sizes_field = sizes_field.unwrap();
            let (width, height) = size.dimensions();
            let expected_sizes = format!("{}x{}", width, height);

            prop_assert_eq!(
                sizes_field,
                &expected_sizes,
                "Icon sizes field should be '{}' for {:?}",
                expected_sizes,
                size
            );

            // Verify format is "WxH"
            let parts: Vec<&str> = sizes_field.split('x').collect();
            prop_assert_eq!(
                parts.len(),
                2,
                "Sizes field should be in 'WxH' format"
            );

            // Verify both parts are valid numbers
            prop_assert!(
                parts[0].parse::<u32>().is_ok(),
                "Width should be a valid number"
            );
            prop_assert!(
                parts[1].parse::<u32>().is_ok(),
                "Height should be a valid number"
            );
        }
    }

    /// Property 2d: Each manifest reference has correct type field
    ///
    /// For any generated favicon set, each icon entry in the manifest.json
    /// should have a 'type' field with the correct MIME type.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn manifest_references_have_correct_type(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: sizes.clone(),
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Generate manifest JSON
        let json_result = processor.generate_manifest_json(&manifest);
        prop_assert!(json_result.is_ok(), "Manifest JSON generation should succeed");

        let json_str = json_result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let icons = json["icons"].as_array().unwrap();

        // Verify each icon entry has correct type field
        for (size, icon) in sizes.iter().zip(icons.iter()) {
            let type_field = icon["type"].as_str();
            prop_assert!(
                type_field.is_some(),
                "Icon entry should have 'type' field"
            );

            let type_field = type_field.unwrap();
            let expected_mime = match size.format() {
                "ico" => "image/x-icon",
                "png" => "image/png",
                _ => "application/octet-stream",
            };

            prop_assert_eq!(
                type_field,
                expected_mime,
                "Icon type field should be '{}' for {:?}",
                expected_mime,
                size
            );

            // Verify MIME type format
            prop_assert!(
                type_field.contains('/'),
                "MIME type should contain '/'"
            );

            let parts: Vec<&str> = type_field.split('/').collect();
            prop_assert_eq!(
                parts.len(),
                2,
                "MIME type should have format 'type/subtype'"
            );
        }
    }

    /// Property 2e: Manifest JSON is valid and parseable
    ///
    /// For any generated favicon set, the manifest.json should be valid JSON
    /// that can be parsed and contains the required PWA manifest fields.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn manifest_json_is_valid_and_parseable(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: sizes.clone(),
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Generate manifest JSON
        let json_result = processor.generate_manifest_json(&manifest);
        prop_assert!(json_result.is_ok(), "Manifest JSON generation should succeed");

        let json_str = json_result.unwrap();

        // Verify JSON is parseable
        let parse_result = serde_json::from_str::<serde_json::Value>(&json_str);
        prop_assert!(
            parse_result.is_ok(),
            "Manifest JSON should be valid and parseable"
        );

        let json = parse_result.unwrap();

        // Verify required PWA manifest fields exist
        prop_assert!(
            json.get("name").is_some(),
            "Manifest should have 'name' field"
        );
        prop_assert!(
            json.get("short_name").is_some(),
            "Manifest should have 'short_name' field"
        );
        prop_assert!(
            json.get("icons").is_some(),
            "Manifest should have 'icons' field"
        );
        prop_assert!(
            json.get("theme_color").is_some(),
            "Manifest should have 'theme_color' field"
        );
        prop_assert!(
            json.get("background_color").is_some(),
            "Manifest should have 'background_color' field"
        );
        prop_assert!(
            json.get("display").is_some(),
            "Manifest should have 'display' field"
        );

        // Verify icons is an array
        prop_assert!(
            json["icons"].is_array(),
            "Manifest 'icons' field should be an array"
        );

        let icons = json["icons"].as_array().unwrap();
        prop_assert_eq!(
            icons.len(),
            sizes.len(),
            "Manifest should have {} icon entries",
            sizes.len()
        );
    }

    /// Property 2f: All referenced files actually exist
    ///
    /// For any generated favicon set, every file referenced in the manifest.json
    /// should actually exist on the filesystem.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn all_referenced_files_exist(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: sizes.clone(),
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Generate manifest JSON
        let json_result = processor.generate_manifest_json(&manifest);
        prop_assert!(json_result.is_ok(), "Manifest JSON generation should succeed");

        let json_str = json_result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let icons = json["icons"].as_array().unwrap();

        // Verify each referenced file exists
        for icon in icons {
            let src = icon["src"].as_str().unwrap();
            let file_path = output_dir.join(src);

            prop_assert!(
                file_path.exists(),
                "Referenced file should exist: {}",
                src
            );

            // Verify file is not empty
            let metadata = fs::metadata(&file_path).unwrap();
            prop_assert!(
                metadata.len() > 0,
                "Referenced file should not be empty: {}",
                src
            );

            // Verify file is readable
            let read_result = fs::read(&file_path);
            prop_assert!(
                read_result.is_ok(),
                "Referenced file should be readable: {}",
                src
            );
        }
    }

    /// Property 2g: Manifest consistency between internal and JSON representations
    ///
    /// For any generated favicon set, the internal FaviconManifest structure
    /// should match the generated JSON manifest exactly.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn manifest_consistency_internal_and_json(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            sizes: sizes.clone(),
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Generate manifest JSON
        let json_result = processor.generate_manifest_json(&manifest);
        prop_assert!(json_result.is_ok(), "Manifest JSON generation should succeed");

        let json_str = json_result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let icons = json["icons"].as_array().unwrap();

        // Verify internal manifest matches JSON manifest
        prop_assert_eq!(
            manifest.icons.len(),
            icons.len(),
            "Internal and JSON manifests should have same number of icons"
        );

        for (internal_icon, json_icon) in manifest.icons.iter().zip(icons.iter()) {
            // Verify src matches
            prop_assert_eq!(
                &internal_icon.src,
                json_icon["src"].as_str().unwrap(),
                "Internal and JSON src should match"
            );

            // Verify sizes matches
            prop_assert_eq!(
                &internal_icon.sizes,
                json_icon["sizes"].as_str().unwrap(),
                "Internal and JSON sizes should match"
            );

            // Verify type matches
            prop_assert_eq!(
                &internal_icon.mime_type,
                json_icon["type"].as_str().unwrap(),
                "Internal and JSON type should match"
            );
        }
    }
}
