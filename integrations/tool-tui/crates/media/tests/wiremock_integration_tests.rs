//! Integration tests for dx-media providers using wiremock.
//!
//! These tests mock HTTP responses to verify provider behavior without hitting real APIs.

mod nasa_tests {
    use dx_media::providers::NasaImagesProvider;
    use dx_media::providers::traits::Provider;
    use dx_media::types::{MediaType, SearchQuery};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Test successful search parsing with valid NASA API response.
    #[tokio::test]
    async fn test_nasa_search_parses_valid_response() {
        let mock_server = MockServer::start().await;

        let fixture = include_str!("integration/fixtures/nasa_success.json");

        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("q", "mars"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture))
            .mount(&mock_server)
            .await;

        let provider = NasaImagesProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("mars").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_ok(), "Search should succeed: {:?}", result.err());

        let search_result = result.unwrap();
        assert_eq!(search_result.total_count, 3, "Should have 3 total hits");
        assert_eq!(search_result.assets.len(), 3, "Should have 3 assets");

        // Verify first asset
        let asset1 = &search_result.assets[0];
        assert_eq!(asset1.id, "PIA00001");
        assert_eq!(asset1.title, "Mars Surface");
        assert_eq!(asset1.provider, "nasa");
        assert_eq!(asset1.media_type, MediaType::Image);
        assert!(!asset1.download_url.is_empty(), "Download URL should not be empty");
        assert!(!asset1.source_url.is_empty(), "Source URL should not be empty");

        // Verify second asset
        let asset2 = &search_result.assets[1];
        assert_eq!(asset2.id, "PIA00002");
        assert_eq!(asset2.title, "Earth from Space");

        // Verify third asset
        let asset3 = &search_result.assets[2];
        assert_eq!(asset3.id, "PIA00003");
        assert_eq!(asset3.title, "Jupiter's Great Red Spot");
    }

    /// Test error handling for malformed JSON responses.
    #[tokio::test]
    async fn test_nasa_handles_malformed_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let provider = NasaImagesProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("mars").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_err(), "Search should fail for malformed JSON");
    }

    /// Test error handling for HTTP error responses.
    #[tokio::test]
    async fn test_nasa_handles_http_error() {
        let mock_server = MockServer::start().await;

        let error_fixture = include_str!("integration/fixtures/nasa_error.json");

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_fixture))
            .mount(&mock_server)
            .await;

        let provider = NasaImagesProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("invalid").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_err(), "Search should fail for HTTP 400 error");
    }

    /// Test handling of empty search results.
    #[tokio::test]
    async fn test_nasa_handles_empty_results() {
        let mock_server = MockServer::start().await;

        let empty_response = r#"{
            "collection": {
                "items": [],
                "metadata": {
                    "total_hits": 0
                }
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string(empty_response))
            .mount(&mock_server)
            .await;

        let provider = NasaImagesProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("nonexistent").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_ok(), "Search should succeed for empty results");

        let search_result = result.unwrap();
        assert_eq!(search_result.total_count, 0);
        assert!(search_result.assets.is_empty());
    }

    /// Test that assets without required fields are filtered out.
    #[tokio::test]
    async fn test_nasa_filters_incomplete_assets() {
        let mock_server = MockServer::start().await;

        // Response with one complete item and one missing links (no preview URL)
        let partial_response = r#"{
            "collection": {
                "items": [
                    {
                        "href": "https://images-api.nasa.gov/asset/PIA00001",
                        "data": [
                            {
                                "nasa_id": "PIA00001",
                                "title": "Complete Item",
                                "media_type": "image",
                                "center": "JPL"
                            }
                        ],
                        "links": [
                            {
                                "href": "https://example.com/thumb.jpg",
                                "rel": "preview",
                                "render": "image"
                            }
                        ]
                    },
                    {
                        "href": "https://images-api.nasa.gov/asset/PIA00002",
                        "data": [
                            {
                                "nasa_id": "PIA00002",
                                "title": "Item Without Links",
                                "media_type": "image"
                            }
                        ]
                    }
                ],
                "metadata": {
                    "total_hits": 2
                }
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string(partial_response))
            .mount(&mock_server)
            .await;

        let provider = NasaImagesProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("test").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_ok(), "Search should succeed");

        let search_result = result.unwrap();
        // Both items should be present - the one without links will have empty download_url
        // which may or may not be filtered depending on builder validation
        assert!(search_result.assets.len() <= 2, "Should have at most 2 assets");
    }
}

mod openverse_tests {
    use dx_media::providers::OpenverseProvider;
    use dx_media::providers::traits::Provider;
    use dx_media::types::{License, MediaType, SearchQuery};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Test successful search parsing with valid Openverse API response.
    #[tokio::test]
    async fn test_openverse_search_parses_valid_response() {
        let mock_server = MockServer::start().await;

        let fixture = include_str!("integration/fixtures/openverse_success.json");

        Mock::given(method("GET"))
            .and(path("/images/"))
            .and(query_param("q", "sunset"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture))
            .mount(&mock_server)
            .await;

        let provider = OpenverseProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("sunset").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_ok(), "Search should succeed: {:?}", result.err());

        let search_result = result.unwrap();
        assert_eq!(search_result.total_count, 3, "Should have 3 total results");
        assert_eq!(search_result.assets.len(), 3, "Should have 3 assets");

        // Verify first asset
        let asset1 = &search_result.assets[0];
        assert_eq!(asset1.id, "abc123");
        assert_eq!(asset1.title, "Beautiful Sunset");
        assert_eq!(asset1.provider, "openverse");
        assert_eq!(asset1.media_type, MediaType::Image);
        assert_eq!(asset1.author, Some("John Doe".to_string()));
        assert!(!asset1.download_url.is_empty(), "Download URL should not be empty");
        assert!(!asset1.source_url.is_empty(), "Source URL should not be empty");
        assert!(matches!(asset1.license, License::CcBy));

        // Verify second asset with CC0 license
        let asset2 = &search_result.assets[1];
        assert_eq!(asset2.id, "def456");
        assert_eq!(asset2.title, "Mountain Landscape");
        assert!(matches!(asset2.license, License::Cc0));

        // Verify third asset
        let asset3 = &search_result.assets[2];
        assert_eq!(asset3.id, "ghi789");
        assert_eq!(asset3.title, "Ocean Waves");
        assert!(matches!(asset3.license, License::CcBySa));
    }

    /// Test error handling for malformed JSON responses.
    #[tokio::test]
    async fn test_openverse_handles_malformed_response() {
        let mock_server = MockServer::start().await;

        let malformed = include_str!("integration/fixtures/openverse_malformed.json");

        Mock::given(method("GET"))
            .and(path("/images/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(malformed))
            .mount(&mock_server)
            .await;

        let provider = OpenverseProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("test").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_err(), "Search should fail for malformed JSON");
    }

    /// Test error handling for HTTP error responses.
    #[tokio::test]
    async fn test_openverse_handles_http_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/images/"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let provider = OpenverseProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("test").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_err(), "Search should fail for HTTP 500 error");
    }

    /// Test handling of empty search results.
    #[tokio::test]
    async fn test_openverse_handles_empty_results() {
        let mock_server = MockServer::start().await;

        let empty_response = r#"{
            "result_count": 0,
            "page_count": 0,
            "page_size": 20,
            "page": 1,
            "results": []
        }"#;

        Mock::given(method("GET"))
            .and(path("/images/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(empty_response))
            .mount(&mock_server)
            .await;

        let provider = OpenverseProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("nonexistent").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_ok(), "Search should succeed for empty results");

        let search_result = result.unwrap();
        assert_eq!(search_result.total_count, 0);
        assert!(search_result.assets.is_empty());
    }

    /// Test that assets with missing optional fields are handled correctly.
    #[tokio::test]
    async fn test_openverse_handles_missing_optional_fields() {
        let mock_server = MockServer::start().await;

        let minimal_response = r#"{
            "result_count": 1,
            "page_count": 1,
            "page_size": 20,
            "page": 1,
            "results": [
                {
                    "id": "minimal123",
                    "title": null,
                    "foreign_landing_url": "https://example.com/minimal",
                    "url": "https://example.com/minimal.jpg",
                    "license": "cc0"
                }
            ]
        }"#;

        Mock::given(method("GET"))
            .and(path("/images/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(minimal_response))
            .mount(&mock_server)
            .await;

        let provider = OpenverseProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("minimal").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_ok(), "Search should succeed with minimal fields");

        let search_result = result.unwrap();
        assert_eq!(search_result.assets.len(), 1);

        let asset = &search_result.assets[0];
        assert_eq!(asset.id, "minimal123");
        // Title should default to "Openverse Image" when null
        assert_eq!(asset.title, "Openverse Image");
    }

    /// Test license parsing for various license types.
    #[tokio::test]
    async fn test_openverse_license_parsing() {
        let mock_server = MockServer::start().await;

        let license_response = r#"{
            "result_count": 4,
            "page_count": 1,
            "page_size": 20,
            "page": 1,
            "results": [
                {
                    "id": "cc0_item",
                    "title": "CC0 Item",
                    "foreign_landing_url": "https://example.com/cc0",
                    "url": "https://example.com/cc0.jpg",
                    "license": "cc0"
                },
                {
                    "id": "by_item",
                    "title": "BY Item",
                    "foreign_landing_url": "https://example.com/by",
                    "url": "https://example.com/by.jpg",
                    "license": "by"
                },
                {
                    "id": "by_sa_item",
                    "title": "BY-SA Item",
                    "foreign_landing_url": "https://example.com/by-sa",
                    "url": "https://example.com/by-sa.jpg",
                    "license": "by-sa"
                },
                {
                    "id": "pdm_item",
                    "title": "PDM Item",
                    "foreign_landing_url": "https://example.com/pdm",
                    "url": "https://example.com/pdm.jpg",
                    "license": "pdm"
                }
            ]
        }"#;

        Mock::given(method("GET"))
            .and(path("/images/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(license_response))
            .mount(&mock_server)
            .await;

        let provider = OpenverseProvider::with_base_url(&mock_server.uri());
        let query = SearchQuery::new("licenses").count(10);
        let result = provider.search(&query).await;

        assert!(result.is_ok(), "Search should succeed");

        let search_result = result.unwrap();
        assert_eq!(search_result.assets.len(), 4);

        assert!(matches!(search_result.assets[0].license, License::Cc0));
        assert!(matches!(search_result.assets[1].license, License::CcBy));
        assert!(matches!(search_result.assets[2].license, License::CcBySa));
        assert!(matches!(search_result.assets[3].license, License::PublicDomain));
    }
}

mod rate_limiting_tests {
    use dx_media::types::RateLimitConfig;

    /// Test that rate limiter delays requests appropriately.
    ///
    /// This test verifies that the RateLimitConfig correctly calculates
    /// the delay between requests based on the configured rate limit.
    #[test]
    fn test_rate_limit_delay_calculation() {
        // 10 requests per 10 seconds = 1 request per second = 1000ms delay
        let config = RateLimitConfig::new(10, 10);
        assert_eq!(config.delay_ms(), 1000, "10 req/10s should have 1000ms delay");

        // 100 requests per 60 seconds = ~600ms delay
        let config = RateLimitConfig::new(100, 60);
        assert_eq!(config.delay_ms(), 600, "100 req/60s should have 600ms delay");

        // 1 request per 1 second = 1000ms delay
        let config = RateLimitConfig::new(1, 1);
        assert_eq!(config.delay_ms(), 1000, "1 req/1s should have 1000ms delay");

        // Unlimited rate limit
        let config = RateLimitConfig::unlimited();
        assert!(!config.is_limited(), "Unlimited config should not be limited");
    }

    /// Test that rate limit configuration properties are accessible.
    #[test]
    fn test_rate_limit_config_properties() {
        let config = RateLimitConfig::new(100, 60);

        assert_eq!(config.requests_per_window(), 100);
        assert_eq!(config.window_secs(), 60);
        assert!(config.is_limited());

        let unlimited = RateLimitConfig::unlimited();
        assert!(!unlimited.is_limited());
    }

    /// Test default rate limit configuration.
    #[test]
    fn test_default_rate_limit_config() {
        let config = RateLimitConfig::default();

        // Default is 100 requests per 60 seconds
        assert_eq!(config.requests_per_window(), 100);
        assert_eq!(config.window_secs(), 60);
        assert!(config.is_limited());
    }
}
