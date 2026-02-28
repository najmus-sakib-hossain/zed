//! Property tests for favicon generation
//!
//! Feature: dx-www-production-ready, Property 1: Favicon Generation Completeness
//!
//! This property test verifies that for any valid source logo image, generating favicons
//! produces all required sizes and formats. The property tests that:
//! 1. All configured sizes are generated
//! 2. Each generated file exists
//! 3. Each file has the correct format (ICO or PNG)
//! 4. The manifest includes all generated icons

use build::{BuildCache, FaviconSize, MediaConfig, MediaProcessor};
use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Strategy for generating arbitrary image content
/// (In reality, this would be valid PNG data, but for testing we use arbitrary bytes)
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
    /// Property 1a: All configured sizes are generated
    ///
    /// For any valid source logo image and any set of configured sizes,
    /// generating favicons should produce exactly those sizes.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn all_configured_sizes_generated(
        image_content in arbitrary_image_content(),
        sizes in arbitrary_favicon_sizes(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Configure media processor with arbitrary sizes
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

        let (manifest, artifacts) = result.unwrap();

        // Verify all configured sizes are in the manifest
        prop_assert_eq!(
            manifest.icons.len(),
            sizes.len(),
            "Manifest should contain exactly {} icons",
            sizes.len()
        );

        // Verify all configured sizes are in the artifacts
        prop_assert_eq!(
            artifacts.len(),
            sizes.len(),
            "Should generate exactly {} artifacts",
            sizes.len()
        );

        // Verify each configured size has a corresponding manifest entry
        for size in &sizes {
            let expected_filename = size.filename();
            let found = manifest.icons.iter().any(|icon| icon.src == expected_filename);
            prop_assert!(
                found,
                "Manifest should contain entry for {}",
                expected_filename
            );
        }
    }

    /// Property 1b: Each generated file exists
    ///
    /// For any valid source logo image, all generated favicon files should exist
    /// on the filesystem.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn all_generated_files_exist(
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

        // Verify each file in the manifest exists
        for icon in &manifest.icons {
            let file_path = output_dir.join(&icon.src);
            prop_assert!(
                file_path.exists(),
                "Generated file should exist: {}",
                icon.src
            );

            // Verify file is not empty
            let metadata = fs::metadata(&file_path).unwrap();
            prop_assert!(
                metadata.len() > 0,
                "Generated file should not be empty: {}",
                icon.src
            );
        }
    }

    /// Property 1c: Each file has the correct format
    ///
    /// For any valid source logo image, each generated favicon should have the
    /// correct MIME type (ICO or PNG) based on its size specification.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn correct_format_for_each_size(
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

        // Verify each icon has the correct MIME type
        for (size, icon) in sizes.iter().zip(manifest.icons.iter()) {
            let expected_mime = match size.format() {
                "ico" => "image/x-icon",
                "png" => "image/png",
                _ => "application/octet-stream",
            };

            prop_assert_eq!(
                &icon.mime_type,
                expected_mime,
                "Icon {} should have MIME type {}",
                icon.src,
                expected_mime
            );

            // Verify filename extension matches format
            let expected_ext = size.format();
            prop_assert!(
                icon.src.ends_with(&format!(".{}", expected_ext)),
                "Icon {} should have extension .{}",
                icon.src,
                expected_ext
            );
        }
    }

    /// Property 1d: Manifest includes all generated icons
    ///
    /// For any valid source logo image, the manifest should include entries for
    /// all generated icons with correct sizes and MIME types.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn manifest_includes_all_icons(
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

        // Verify manifest has correct number of entries
        prop_assert_eq!(
            manifest.icons.len(),
            sizes.len(),
            "Manifest should have {} entries",
            sizes.len()
        );

        // Verify each size has a corresponding manifest entry with correct dimensions
        for size in &sizes {
            let (width, height) = size.dimensions();
            let expected_sizes = format!("{}x{}", width, height);

            let found = manifest.icons.iter().any(|icon| {
                icon.src == size.filename() && icon.sizes == expected_sizes
            });

            prop_assert!(
                found,
                "Manifest should contain entry for {} with sizes {}",
                size.filename(),
                expected_sizes
            );
        }
    }

    /// Property 1e: Default configuration generates all required sizes
    ///
    /// Using the default MediaConfig, all 8 required favicon sizes should be generated:
    /// 16x16, 32x32, 48x48 (ICO), 16x16, 32x32 (PNG), 180x180 (Apple Touch),
    /// 192x192, 512x512 (Android Chrome).
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn default_config_generates_all_required_sizes(
        image_content in arbitrary_image_content(),
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create source logo file
        let logo_path = temp_dir.path().join("logo.png");
        fs::write(&logo_path, &image_content).unwrap();

        // Use default config but override paths
        let output_dir = temp_dir.path().join("favicons");
        let config = MediaConfig {
            logo_path: logo_path.clone(),
            output_dir: output_dir.clone(),
            ..Default::default()
        };

        let processor = MediaProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Generate favicons
        let result = processor.generate_favicons(&mut cache);
        prop_assert!(result.is_ok(), "Favicon generation should succeed");

        let (manifest, _) = result.unwrap();

        // Verify all 8 required sizes are generated
        prop_assert_eq!(
            manifest.icons.len(),
            8,
            "Default config should generate 8 favicon sizes"
        );

        // Verify specific required sizes exist
        let required_filenames = vec![
            "favicon-16x16.ico",
            "favicon-32x32.ico",
            "favicon-48x48.ico",
            "favicon-16x16.png",
            "favicon-32x32.png",
            "apple-touch-icon.png",
            "android-chrome-192x192.png",
            "android-chrome-512x512.png",
        ];

        for filename in required_filenames {
            let found = manifest.icons.iter().any(|icon| icon.src == filename);
            prop_assert!(
                found,
                "Default config should generate {}",
                filename
            );

            // Verify file exists
            let file_path = output_dir.join(filename);
            prop_assert!(
                file_path.exists(),
                "Required file should exist: {}",
                filename
            );
        }
    }

    /// Property 1f: Manifest JSON generation includes all icons
    ///
    /// For any generated favicon manifest, the JSON output should include all icons
    /// with correct structure.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn manifest_json_includes_all_icons(
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

        let json = json_result.unwrap();

        // Verify JSON contains all icon filenames
        for icon in &manifest.icons {
            prop_assert!(
                json.contains(&icon.src),
                "Manifest JSON should contain {}",
                icon.src
            );
        }

        // Verify JSON is valid
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        prop_assert!(
            parsed.get("icons").is_some(),
            "Manifest JSON should have 'icons' field"
        );

        let icons_array = parsed["icons"].as_array().unwrap();
        prop_assert_eq!(
            icons_array.len(),
            manifest.icons.len(),
            "Manifest JSON should have {} icons",
            manifest.icons.len()
        );
    }
}
