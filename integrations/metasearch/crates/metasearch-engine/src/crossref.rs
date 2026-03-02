//! Crossref engine — search scholarly publications via Crossref API.
//! Translated from SearXNG `searx/engines/crossref.py`.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Crossref {
    metadata: EngineMetadata,
    client: Client,
}

impl Crossref {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "crossref".to_string().into(),
                display_name: "Crossref".to_string().into(),
                homepage: "https://www.crossref.org".to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Crossref {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let offset = 20 * (page - 1);

        let url = format!(
            "https://api.crossref.org/works?query={}&offset={}",
            urlencoding::encode(&query.query),
            offset,
        );

        let resp = match self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header("User-Agent", "metasearch-engine/1.0 (https://github.com/najmus-sakib-hossain/metasearch; mailto:metasearch@example.com)")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match resp
            .json()
            .await
        {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(items) = data["message"]["items"].as_array() {
            for (i, record) in items.iter().enumerate() {
                let record_type = record["type"].as_str().unwrap_or("");
                if record_type == "component" {
                    continue;
                }

                // Title extraction
                let title = if record_type == "book-chapter" {
                    let container = record["container-title"][0].as_str().unwrap_or("");
                    let chapter = record["title"][0].as_str().unwrap_or("");
                    if !chapter.is_empty()
                        && chapter.to_lowercase().trim() != container.to_lowercase().trim()
                    {
                        format!("{} ({})", container, chapter)
                    } else {
                        container.to_string()
                    }
                } else {
                    record["title"][0]
                        .as_str()
                        .or_else(|| record["container-title"][0].as_str())
                        .unwrap_or("Untitled")
                        .to_string()
                };

                // URL — prefer resource primary URL
                let url = record["resource"]["primary"]["URL"]
                    .as_str()
                    .or_else(|| record["URL"].as_str())
                    .unwrap_or_default();

                // Authors
                let authors: Vec<String> = record["author"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|a| {
                        let given = a["given"].as_str().unwrap_or("");
                        let family = a["family"].as_str().unwrap_or("");
                        format!("{} {}", given, family).trim().to_string()
                    })
                    .collect();

                let doi = record["DOI"].as_str().unwrap_or("");
                let publisher = record["publisher"].as_str().unwrap_or("");
                let journal = record["container-title"][0].as_str().unwrap_or("");

                let mut content_parts = Vec::new();
                if !authors.is_empty() {
                    content_parts.push(authors.join(", "));
                }
                if !journal.is_empty() {
                    content_parts.push(journal.to_string());
                }
                if !publisher.is_empty() {
                    content_parts.push(publisher.to_string());
                }
                if !doi.is_empty() {
                    content_parts.push(format!("DOI: {}", doi));
                }
                content_parts.push(format!("Type: {}", record_type));

                let snippet = content_parts.join(" | ");

                let mut result =
                    SearchResult::new(title, url.to_string(), snippet, "crossref".to_string());
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Science.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
