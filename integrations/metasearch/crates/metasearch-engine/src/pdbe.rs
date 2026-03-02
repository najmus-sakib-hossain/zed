//! PDBe (Protein Data Bank Europe) search engine implementation.
//!
//! Queries the PDBe search API for protein structure entries.
//! Website: https://www.ebi.ac.uk/pdbe
//! Features: Paging

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

const PDBE_BASE: &str = "https://www.ebi.ac.uk/pdbe";

pub struct Pdbe {
    metadata: EngineMetadata,
    client: Client,
}

impl Pdbe {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "pdbe".to_string().into(),
                display_name: "PDBe".to_string().into(),
                homepage: PDBE_BASE.to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Pdbe {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let rows = 20;
        let start = (page - 1) * rows;

        let resp = match self
            .client
            .post(format!("{}/search/pdb/select", PDBE_BASE))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(7))
            .body(format!(
                "q={}&wt=json&rows={}&start={}",
                urlencoding::encode(&query.query),
                rows,
                start
            ))
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
            Ok(d) => d,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        let docs = data["response"]["docs"].as_array();
        if let Some(docs) = docs {
            for (i, doc) in docs.iter().enumerate() {
                let pdb_id = doc["pdb_id"].as_str().unwrap_or_default();
                if pdb_id.is_empty() {
                    continue;
                }

                let item_url = format!("{}/entry/pdb/{}", PDBE_BASE, pdb_id);

                let title = doc["title"].as_str().unwrap_or(pdb_id);

                // Build content from available fields
                let mut content_parts: Vec<String> = Vec::new();

                if let Some(citation_title) = doc["citation_title"].as_str() {
                    if !citation_title.is_empty() {
                        content_parts.push(citation_title.to_string());
                    }
                }
                if let Some(journal) = doc["journal"].as_str() {
                    if !journal.is_empty() {
                        content_parts.push(journal.to_string());
                    }
                }
                if let Some(authors) = doc["entry_author_list"].as_array() {
                    let author_strs: Vec<&str> = authors
                        .iter()
                        .filter_map(|a| a.as_str())
                        .collect();
                    if !author_strs.is_empty() {
                        content_parts.push(author_strs.join(", "));
                    }
                } else if let Some(authors) = doc["entry_author_list"].as_str() {
                    if !authors.is_empty() {
                        content_parts.push(authors.to_string());
                    }
                }

                let content = content_parts.join(" | ");

                let thumbnail = format!(
                    "{}/static/entry/{}_deposited_chain_front_image-200x200.png",
                    PDBE_BASE, pdb_id
                );

                let mut r = SearchResult::new(title, &item_url, &content, "pdbe");
                r.engine_rank = i as u32;
                r.category = SearchCategory::Science.to_string();
                r.thumbnail = Some(thumbnail);
                results.push(r);
            }
        }

        Ok(results)
    }
}
