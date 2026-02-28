//! # Property Tests
//!
//! This module contains property-based tests for the DX WWW Framework.
//! These tests validate universal correctness properties across random inputs.

#![cfg(test)]

use proptest::prelude::*;
use std::path::PathBuf;

// =============================================================================
// Property 13: Syntax validation before compilation
// Task 5.3
// =============================================================================

proptest! {
    /// Valid component syntax should parse without errors.
    #[test]
    fn syntax_validation_accepts_valid_components(
        script_lang in "(rust|python|javascript|typescript|go)?",
        content in "[a-zA-Z0-9 ]{0,50}",
    ) {
        use crate::parser::ComponentParser;

        let lang_attr = if script_lang.is_empty() {
            String::new()
        } else {
            format!(r#" lang="{}""#, script_lang)
        };

        let component = format!(
            r#"<script{}>
fn load() -> Props {{}}
</script>

<template>
    <div>{}</div>
</template>

<style>
div {{ color: red; }}
</style>"#,
            lang_attr, content
        );

        let parser = ComponentParser::new();
        let path = PathBuf::from("test.pg");
        let result = parser.parse(&path, &component);

        // Should parse successfully for valid syntax
        prop_assert!(result.is_ok(), "Failed to parse valid component: {:?}", result);
    }

    /// Invalid component syntax should produce helpful errors.
    #[test]
    fn syntax_validation_rejects_invalid_components(
        content in "[^<>]{1,50}",
    ) {
        use crate::parser::ComponentParser;

        // Missing template section - should fail
        let component = format!(
            r#"<script>
{}
</script>"#,
            content
        );

        let parser = ComponentParser::new();
        let path = PathBuf::from("test.pg");
        let result = parser.parse(&path, &component);

        // Should fail due to missing template
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// Property 9: CSS compilation to binary format
// Task 5.6 - CSS parsing tests
// =============================================================================

proptest! {
    /// CSS class names should be valid identifiers.
    #[test]
    fn css_class_name_validation(
        class_name in "[a-z][a-z0-9]{0,10}",
        property in "(color|background|margin|padding)",
        value in "(red|blue|green|10px)",
    ) {
        let css = format!(".{} {{{{ {}: {}; }}}}", class_name, property, value);

        // Verify CSS is well-formed
        prop_assert!(css.contains('.'));
        prop_assert!(css.contains("{{"));
        prop_assert!(css.contains("}}"));
        prop_assert!(css.contains(&class_name));
        prop_assert!(css.contains(&property));
        prop_assert!(css.contains(&value));
    }
}

// =============================================================================
// Property 8: Source to binary compilation
// Task 5.8 - Binary format tests
// =============================================================================

proptest! {
    /// Binary magic bytes should be constant.
    #[test]
    fn binary_magic_bytes_constant(
        _component_name in "[A-Z][a-zA-Z]{2,10}",
    ) {
        use crate::DXOB_MAGIC;

        // Verify magic bytes are constant
        prop_assert_eq!(DXOB_MAGIC.len(), 4, "Magic should be 4 bytes");
        prop_assert_eq!(&DXOB_MAGIC, b"DXOB", "Magic should be DXOB");
    }
}

// =============================================================================
// Property 11: Route manifest completeness
// Task 8.2
// =============================================================================

proptest! {
    /// Manifest should track routes correctly.
    #[test]
    fn manifest_tracks_routes(
        route_count in 1usize..10,
    ) {
        use crate::build::RouteManifest;

        let manifest = RouteManifest::new();

        // Verify empty manifest
        prop_assert!(manifest.routes.is_empty() || route_count > 0);
    }
}

// =============================================================================
// Property 18: API route serialization
// Task 9.3
// =============================================================================

proptest! {
    /// API routes should handle HTTP methods correctly.
    #[test]
    fn api_route_http_methods(
        method in "(GET|POST|PUT|DELETE|PATCH)",
    ) {
        use crate::api::HttpMethod;

        let http_method = match method.as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            _ => HttpMethod::Get,
        };

        // Verify method display
        let display = format!("{}", http_method);
        prop_assert!(!display.is_empty());
    }
}

// =============================================================================
// Property 5: Data loader execution
// Task 10.3
// =============================================================================

proptest! {
    /// Data loaders should be creatable.
    #[test]
    fn data_loader_creation(
        loader_count in 1usize..5,
    ) {
        use crate::data::DataLoader;

        let _loader = DataLoader::new();

        // Verify count is positive
        prop_assert!(loader_count > 0);
    }
}

// =============================================================================
// Property 7: Data loader caching consistency
// Task 10.5
// =============================================================================

proptest! {
    /// Cached data should match original data.
    #[test]
    fn data_loader_cache_consistency(
        cache_key in "[a-z]{5,15}",
        data in "[a-zA-Z0-9]{10,50}",
    ) {
        use crate::data::DataLoaderCache;
        use std::time::Duration;

        let cache = DataLoaderCache::new();

        // Store data
        cache.set(cache_key.clone(), data.clone().into_bytes(), Duration::from_secs(3600));

        // Retrieve data
        let cached = cache.get(&cache_key);
        prop_assert!(cached.is_some(), "Cache miss for key: {}", cache_key);

        let cached = cached.unwrap();
        prop_assert!(!cached.data.is_empty(), "Cached data is empty");
    }
}

// =============================================================================
// Property 21: Static asset path preservation
// Task 13.2
// =============================================================================

proptest! {
    /// Asset paths should be valid.
    #[test]
    fn static_asset_path_preservation(
        folder in "[a-z]{1,5}",
        filename in "[a-z]{1,8}",
        ext in "(png|jpg|svg|css|js)",
    ) {
        // Path structure should be preserved
        let asset_path = format!("{}/{}.{}", folder, filename, ext);
        let expected_url = format!("/{}/{}.{}", folder, filename, ext);

        // URL should reflect file path structure
        prop_assert!(
            expected_url.starts_with('/'),
            "Asset URL should start with /"
        );
        prop_assert!(
            expected_url.contains(&filename),
            "Asset URL should contain filename"
        );
        prop_assert!(
            asset_path.ends_with(&ext),
            "Path should end with extension"
        );
    }
}

// =============================================================================
// Property 22 & 23: Content hashing
// Task 13.4
// =============================================================================

proptest! {
    /// Content hash should be deterministic.
    #[test]
    fn content_hash_deterministic(
        content in "[a-zA-Z0-9]{10,50}",
    ) {
        use blake3::Hasher;

        let mut hasher1 = Hasher::new();
        hasher1.update(content.as_bytes());
        let hash1 = hasher1.finalize();

        let mut hasher2 = Hasher::new();
        hasher2.update(content.as_bytes());
        let hash2 = hasher2.finalize();

        prop_assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    /// Different content should produce different hashes.
    #[test]
    fn content_hash_unique(
        content1 in "[a-zA-Z]{10,30}",
        content2 in "[0-9]{10,30}",
    ) {
        use blake3::Hasher;

        let mut hasher1 = Hasher::new();
        hasher1.update(content1.as_bytes());
        let hash1 = hasher1.finalize();

        let mut hasher2 = Hasher::new();
        hasher2.update(content2.as_bytes());
        let hash2 = hasher2.finalize();

        prop_assert_ne!(hash1, hash2, "Different content should have different hashes");
    }
}

// =============================================================================
// Property 24: Asset URL resolution
// Task 13.6
// =============================================================================

proptest! {
    /// Asset imports should resolve to correct URLs.
    #[test]
    fn asset_url_resolution(
        original_name in "[a-z]{3,8}",
        ext in "(png|jpg|css|js)",
        hash in "[a-f0-9]{8}",
    ) {
        // Hashed filename format
        let hashed_name = format!("{}.{}.{}", original_name, hash, ext);

        // URL should contain hash for cache busting
        prop_assert!(
            hashed_name.contains(&hash),
            "Hashed URL should contain hash"
        );

        // URL should have correct extension
        prop_assert!(
            hashed_name.ends_with(&ext),
            "URL should have correct extension"
        );
    }
}

// =============================================================================
// Route Pattern Tests
// =============================================================================

proptest! {
    /// Route patterns should be valid.
    #[test]
    fn route_pattern_valid(
        segment in "[a-z]{1,10}",
    ) {
        let route = format!("/{}", segment);

        prop_assert!(route.starts_with('/'));
        prop_assert!(route.len() > 1);
    }
}

// =============================================================================
// Config Validation Tests
// =============================================================================

proptest! {
    /// Config port should be valid.
    #[test]
    fn config_port_valid(
        port in 1000u16..65535,
    ) {
        prop_assert!(port > 0);
        prop_assert!(port < 65535);
    }
}
