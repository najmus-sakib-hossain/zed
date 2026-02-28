//! Data.gov provider implementation.
//!
//! [Data.gov](https://data.gov) - US Government Open Data Portal
//!
//! Provides free access to 300,000+ datasets from the US Government.
//! No API key required. Includes JSON, CSV, XML, and other data formats.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Data.gov provider for US Government open data.
/// Access 300,000+ datasets including JSON, CSV, XML files.
#[derive(Debug)]
pub struct DataGovProvider {
    client: HttpClient,
}

impl DataGovProvider {
    /// Create a new Data.gov provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self { client }
    }

    /// Rate limit: Be respectful - 30 requests/minute
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(30, 60);

    /// Base URL for Data.gov CKAN API
    const BASE_URL: &'static str = "https://catalog.data.gov/api/3";

    /// Parse format to media type
    fn format_to_media_type(format: &str) -> MediaType {
        match format.to_lowercase().as_str() {
            "json" | "geojson" | "api" => MediaType::Data,
            "csv" | "tsv" | "xlsx" | "xls" => MediaType::Data,
            "xml" | "rss" | "atom" => MediaType::Data,
            "pdf" | "doc" | "docx" => MediaType::Document,
            "txt" | "html" | "htm" => MediaType::Text,
            "zip" | "gz" | "tar" => MediaType::Data,
            "kml" | "kmz" | "shp" => MediaType::Data, // Geographic data
            _ => MediaType::Data,
        }
    }
}

#[async_trait]
impl Provider for DataGovProvider {
    fn name(&self) -> &'static str {
        "datagov"
    }

    fn display_name(&self) -> &'static str {
        "Data.gov"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Data, MediaType::Document, MediaType::Text]
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn rate_limit(&self) -> RateLimitConfig {
        Self::RATE_LIMIT
    }

    fn is_available(&self) -> bool {
        true
    }

    fn base_url(&self) -> &'static str {
        Self::BASE_URL
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let url = format!("{}/action/package_search", Self::BASE_URL);

        let rows = query.count.min(100).to_string();
        let start = ((query.page - 1) * query.count).to_string();

        let params = [
            ("q", query.query.as_str()),
            ("rows", &rows),
            ("start", &start),
        ];

        let response = self.client.get_with_query(&url, &params, &[]).await?;
        let api_response: DataGovResponse = response.json_or_error().await?;

        let mut assets: Vec<MediaAsset> = Vec::new();

        for package in api_response.result.results {
            // Each package can have multiple resources (files)
            for resource in package.resources {
                // Skip resources without URLs
                let download_url = match &resource.url {
                    Some(url) if !url.is_empty() => url.clone(),
                    _ => continue,
                };

                let format = resource.format.as_deref().unwrap_or("unknown");
                let media_type = Self::format_to_media_type(format);

                // Filter by media type if specified
                if let Some(requested_type) = query.media_type {
                    if media_type != requested_type {
                        continue;
                    }
                }

                let id = format!("datagov_{}", resource.id);
                let title = resource
                    .name
                    .clone()
                    .or_else(|| resource.description.clone())
                    .unwrap_or_else(|| package.title.clone());

                let author = package
                    .organization
                    .as_ref()
                    .map(|o| o.title.clone())
                    .unwrap_or_else(|| "US Government".to_string());

                let source_url = format!(
                    "https://catalog.data.gov/dataset/{}",
                    package.name.replace(' ', "-").to_lowercase()
                );

                // Build tags from package tags
                let mut tags: Vec<String> = package.tags.iter().map(|t| t.name.clone()).collect();

                if let Some(fmt) = &resource.format {
                    tags.push(fmt.to_lowercase());
                }
                tags.push("government".to_string());
                tags.push("open-data".to_string());

                let asset = MediaAsset::builder()
                    .id(id)
                    .provider("datagov")
                    .media_type(media_type)
                    .title(title)
                    .download_url(download_url)
                    .source_url(source_url)
                    .author(author)
                    .license(License::PublicDomain)
                    .tags(tags)
                    .build_or_log();

                if let Some(asset) = asset {
                    assets.push(asset);
                }

                // Limit results per package to avoid too many from one dataset
                if assets.len() >= query.count {
                    break;
                }
            }

            if assets.len() >= query.count {
                break;
            }
        }

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.result.count,
            assets,
            providers_searched: vec!["datagov".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for DataGovProvider {
    fn description(&self) -> &'static str {
        "300,000+ US Government open datasets (JSON, CSV, XML, PDF)"
    }

    fn api_key_url(&self) -> &'static str {
        "https://data.gov" // No API key needed
    }

    fn default_license(&self) -> &'static str {
        "Public Domain (US Government)"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// These structs are used by serde for JSON deserialization.
// Fields may appear unused but are read during deserialization.
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct DataGovResponse {
    #[allow(dead_code)] // Read by serde during deserialization
    success: bool,
    result: DataGovResult,
}

#[derive(Debug, Deserialize)]
struct DataGovResult {
    count: usize,
    results: Vec<DataGovPackage>,
}

#[derive(Debug, Deserialize)]
struct DataGovPackage {
    #[allow(dead_code)] // Read by serde during deserialization
    id: String,
    name: String,
    title: String,
    #[serde(default)]
    #[allow(dead_code)] // Read by serde during deserialization
    notes: Option<String>,
    #[serde(default)]
    organization: Option<DataGovOrganization>,
    #[serde(default)]
    resources: Vec<DataGovResource>,
    #[serde(default)]
    tags: Vec<DataGovTag>,
}

#[derive(Debug, Deserialize)]
struct DataGovOrganization {
    #[serde(default)]
    #[allow(dead_code)] // Read by serde during deserialization
    id: String,
    #[serde(default)]
    #[allow(dead_code)] // Read by serde during deserialization
    name: String,
    #[serde(default)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct DataGovResource {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    #[allow(dead_code)] // Read by serde during deserialization
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DataGovTag {
    #[serde(default)]
    #[allow(dead_code)] // Read by serde during deserialization
    id: String,
    name: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let config = Config::default();
        let provider = DataGovProvider::new(&config);

        assert_eq!(provider.name(), "datagov");
        assert_eq!(provider.display_name(), "Data.gov");
        assert!(!provider.requires_api_key());
        assert!(provider.is_available());
    }

    #[test]
    fn test_supported_media_types() {
        let config = Config::default();
        let provider = DataGovProvider::new(&config);

        let types = provider.supported_media_types();
        assert!(types.contains(&MediaType::Data));
        assert!(types.contains(&MediaType::Document));
        assert!(types.contains(&MediaType::Text));
    }

    #[test]
    fn test_format_to_media_type() {
        assert_eq!(DataGovProvider::format_to_media_type("json"), MediaType::Data);
        assert_eq!(DataGovProvider::format_to_media_type("CSV"), MediaType::Data);
        assert_eq!(DataGovProvider::format_to_media_type("xml"), MediaType::Data);
        assert_eq!(DataGovProvider::format_to_media_type("pdf"), MediaType::Document);
        assert_eq!(DataGovProvider::format_to_media_type("txt"), MediaType::Text);
        assert_eq!(DataGovProvider::format_to_media_type("geojson"), MediaType::Data);
    }
}
