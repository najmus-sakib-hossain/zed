//! Semantic Scholar engine — search academic papers via POST API.
//! Translated from SearXNG `searx/engines/semantic_scholar.py`.
//!
//! Uses `semanticscholar.org/api/1/search` (POST with JSON body).
//! Returns papers with titles, abstracts, authors, citation counts,
//! DOIs, and links to PDFs.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

static HTML_TAG_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"<[^>]+>").unwrap());
const SEARCH_URL: &str = "https://www.semanticscholar.org/api/1/search";
const BASE_URL: &str = "https://www.semanticscholar.org";

pub struct SemanticScholar {
    metadata: EngineMetadata,
    client: Client,
}

impl SemanticScholar {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "semantic_scholar".to_string().into(),
                display_name: "Semantic Scholar".to_string().into(),
                homepage: "https://www.semanticscholar.org".to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.2,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for SemanticScholar {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;

        // Build the POST JSON body — mirrors the Python implementation
        let body = serde_json::json!({
            "queryString": query.query,
            "page": page,
            "pageSize": 10,
            "sort": "relevance",
            "getQuerySuggestions": false,
            "authors": [],
            "coAuthors": [],
            "venues": [],
            "performTitleMatch": true,
        });

        let resp = match self
            .client
            .post(SEARCH_URL)
            .timeout(std::time::Duration::from_secs(7))
            .header("Content-Type", "application/json")
            .header("X-S2-Client", "webapp-browser")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(items) = data["results"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let title = item["title"]["text"].as_str().unwrap_or_default();
                if title.is_empty() {
                    continue;
                }

                // URL priority: primaryPaperLink -> links[0] -> alternatePaperLinks[0] -> fallback
                let url = item["primaryPaperLink"]["url"]
                    .as_str()
                    .or_else(|| {
                        item["links"]
                            .as_array()
                            .and_then(|links| links.first())
                            .and_then(|l| l.as_str())
                    })
                    .or_else(|| {
                        item["alternatePaperLinks"]
                            .as_array()
                            .and_then(|links| links.first())
                            .and_then(|l| l["url"].as_str())
                    })
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        let paper_id = item["id"].as_str().unwrap_or("");
                        format!("{}/paper/{}", BASE_URL, paper_id)
                    });

                // Abstract
                let abstract_text = item["paperAbstract"]["text"].as_str().unwrap_or("");
                // Strip any residual HTML tags from the abstract
                let clean_abstract = HTML_TAG_RE.replace_all(abstract_text, "").to_string();

                // Authors (array of [[{"name": "..."}]])
                let authors: Vec<String> = item["authors"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|author_group| {
                        // Each author entry is an array like [{"name": "John Doe"}]
                        author_group
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|a| a["name"].as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();

                // Citation stats
                let citations = item["citationStats"]["numCitations"].as_u64();

                // DOI
                let doi = item["doiInfo"]["doi"].as_str().unwrap_or("");

                // Journal / venue
                let venue = item["venue"]["text"]
                    .as_str()
                    .or_else(|| item["journal"]["name"].as_str())
                    .unwrap_or("");

                // Fields of study (tags)
                let fields: Vec<String> = item["fieldsOfStudy"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|f| f.as_str().map(|s| s.to_string()))
                    .collect();

                // Build snippet
                let mut snippet_parts = Vec::new();
                if !authors.is_empty() {
                    let author_str = if authors.len() > 3 {
                        format!("{} et al.", authors[..3].join(", "))
                    } else {
                        authors.join(", ")
                    };
                    snippet_parts.push(author_str);
                }
                if !venue.is_empty() {
                    snippet_parts.push(venue.to_string());
                }
                if let Some(c) = citations {
                    snippet_parts.push(format!("{} citations", c));
                }
                if !doi.is_empty() {
                    snippet_parts.push(format!("DOI: {}", doi));
                }
                if !fields.is_empty() {
                    snippet_parts.push(fields.join(", "));
                }
                if !clean_abstract.is_empty() {
                    // Truncate abstract to ~250 chars for the snippet
                    let truncated = if clean_abstract.len() > 250 {
                        format!("{}…", &clean_abstract[..250])
                    } else {
                        clean_abstract
                    };
                    snippet_parts.push(truncated);
                }

                let snippet = snippet_parts.join(" | ");

                let mut sr = SearchResult::new(
                    title.to_string(),
                    url,
                    snippet,
                    "semantic_scholar".to_string(),
                );
                sr.engine_rank = (i + 1) as u32;
                sr.category = SearchCategory::Science.to_string();
                results.push(sr);
            }
        }

        Ok(results)
    }
}
