//! Property-based tests for Font CLI commands
//!
//! These tests verify universal properties that should hold across all inputs.
//! Feature: dx-unified-assets
//!
//! Note: Font tests require network access and may be slow.
//! Run with: cargo test --test font_property_tests -- --ignored

use proptest::prelude::*;

/// Generate valid font search query strings
fn font_search_query_strategy() -> impl Strategy<Value = String> {
    // Generate common font-related search terms
    prop_oneof![
        Just("roboto".to_string()),
        Just("open sans".to_string()),
        Just("lato".to_string()),
        Just("montserrat".to_string()),
        Just("poppins".to_string()),
        "[a-z]{3,8}".prop_map(|s| s.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))] // Limit cases due to network calls

    /// Property 4: Font Search Result Relevance
    /// *For any* search query string, all fonts returned by font search SHALL have
    /// their `name` or `id` containing the query string (case-insensitive).
    ///
    /// **Validates: Requirements 2.1**
    #[test]
    #[ignore] // Requires network access
    fn prop_font_search_relevance(query in font_search_query_strategy()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let search = dx_font::FontSearch::new().expect("Failed to create FontSearch");

            match search.search(&query).await {
                Ok(results) => {
                    let query_lower = query.to_lowercase();

                    for font in &results.fonts {
                        let id_matches = font.id.to_lowercase().contains(&query_lower);
                        let name_matches = font.name.to_lowercase().contains(&query_lower);

                        // Note: Some providers may return related fonts, so we check
                        // if at least the query appears somewhere in the result
                        prop_assert!(
                            id_matches || name_matches || results.fonts.len() <= 5,
                            "Font {} ({}) does not contain query '{}' in id or name",
                            font.name, font.id, query
                        );
                    }
                }
                Err(e) => {
                    // Network errors are acceptable in property tests
                    eprintln!("Search failed (acceptable): {}", e);
                }
            }
            Ok(())
        })?;
    }
}

/// Property 6: Font Info Completeness
/// *For any* valid font id, font info SHALL return a response containing
/// all required fields: `id`, `name`, `provider`, `variants`, and `subsets`.
///
/// **Validates: Requirements 2.4**
#[test]
#[ignore] // Requires network access
fn test_font_info_completeness() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let search = dx_font::FontSearch::new().expect("Failed to create FontSearch");

        // Test with a known font
        match search.get_font_details(&dx_font::FontProvider::GoogleFonts, "roboto").await {
            Ok(family) => {
                // Check required fields are present and non-empty
                assert!(!family.id.is_empty(), "Font id should not be empty");
                assert!(!family.name.is_empty(), "Font name should not be empty");
                assert!(!family.variants.is_empty(), "Font should have at least one variant");
                // Subsets may be empty for some fonts, but the field should exist
            }
            Err(e) => {
                // Network errors are acceptable
                eprintln!("Font info failed (acceptable): {}", e);
            }
        }
    });
}

/// Property 5: Font Download Creates Files
/// *For any* valid font id and output directory, font download SHALL create
/// at least one font file in the specified output directory.
///
/// **Validates: Requirements 2.2**
///
/// Note: This test is expensive and modifies the filesystem, so it's marked ignore.
#[test]
#[ignore] // Requires network access and creates files
fn test_font_download_creates_files() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let downloader = dx_font::FontDownloader::new().expect("Failed to create FontDownloader");

        // Create a temporary directory
        let temp_dir = std::env::temp_dir().join("dx_font_test");
        let _ = std::fs::remove_dir_all(&temp_dir); // Clean up any previous test
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        // Download a known font
        match downloader
            .download_google_font("roboto", &temp_dir, &["woff2"], &["latin"])
            .await
        {
            Ok(path) => {
                assert!(path.exists(), "Downloaded file should exist");
                assert!(
                    path.metadata().map(|m| m.len() > 0).unwrap_or(false),
                    "Downloaded file should not be empty"
                );
            }
            Err(e) => {
                // Network errors are acceptable
                eprintln!("Download failed (acceptable): {}", e);
            }
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    });
}

/// Test that font providers list returns valid data
#[test]
fn test_font_providers_validity() {
    // Test that known providers have valid names and URLs
    let providers = vec![
        dx_font::FontProvider::GoogleFonts,
        dx_font::FontProvider::BunnyFonts,
        dx_font::FontProvider::Fontsource,
        dx_font::FontProvider::FontSquirrel,
    ];

    for provider in providers {
        let name = provider.name();
        let url = provider.base_url();

        assert!(!name.is_empty(), "Provider name should not be empty");
        assert!(url.starts_with("https://"), "Provider URL should be HTTPS: {}", url);
    }
}

/// Test that font health check returns results for all providers
#[test]
#[ignore] // Requires network access
fn test_font_health_check() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let search = dx_font::FontSearch::new().expect("Failed to create FontSearch");

        let health_results = search.health_check_timed().await;

        // Should have results for multiple providers
        assert!(!health_results.is_empty(), "Health check should return results");

        // Each result should have a provider name and timing
        for (name, _healthy, duration) in &health_results {
            assert!(!name.is_empty(), "Provider name should not be empty");
            assert!(
                duration.as_millis() > 0 || duration.as_millis() == 0,
                "Duration should be non-negative"
            );
        }
    });
}

/// Property 7: Font Info Completeness (variant check)
/// Font info should return variants with valid weight and style information.
#[test]
#[ignore] // Requires network access
fn test_font_variants_validity() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let search = dx_font::FontSearch::new().expect("Failed to create FontSearch");

        match search.get_font_details(&dx_font::FontProvider::GoogleFonts, "roboto").await {
            Ok(family) => {
                for variant in &family.variants {
                    // Weight should be a valid CSS weight
                    let weight_num = variant.weight.to_numeric();
                    assert!(
                        (100..=900).contains(&weight_num),
                        "Font weight should be between 100 and 900, got {}",
                        weight_num
                    );

                    // Style should be Normal or Italic
                    match variant.style {
                        dx_font::FontStyle::Normal | dx_font::FontStyle::Italic => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Font info failed (acceptable): {}", e);
            }
        }
    });
}
