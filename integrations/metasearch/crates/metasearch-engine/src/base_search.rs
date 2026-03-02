//! BASE (Bielefeld Academic Search Engine) — scholarly publications via XML API.
//!
//! Reference: <https://base-search.net>
//! API docs: <https://api.base-search.net/>

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use smallvec::smallvec;

pub struct BaseSearch {
    metadata: EngineMetadata,
    client: Client,
}

impl BaseSearch {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "base_search".to_string().into(),
                display_name: "BASE".to_string().into(),
                homepage: "https://base-search.net".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 6000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for BaseSearch {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let hits: u32 = 10;
        let offset = (page - 1) * hits;

        let url = format!(
            "https://api.base-search.net/cgi-bin/BaseHttpSearchInterface.fcgi?func=PerformSearch&query={}&boost=oa&hits={}&offset={}",
            urlencoding::encode(&query.query),
            hits,
            offset
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "metasearch/1.0")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body: String = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        // Parse XML: each result is in <doc> with <str name="...">value</str>
        let doc_re = Regex::new(r"(?s)<doc>(.*?)</doc>")
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;
        let field_re = Regex::new(r#"(?s)<str name="([^"]+)">(.*?)</str>"#)
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        for doc_cap in doc_re.captures_iter(&body) {
            let doc_content = &doc_cap[1];
            let mut title = String::new();
            let mut link = String::new();
            let mut content = String::new();

            for field_cap in field_re.captures_iter(doc_content) {
                let name = &field_cap[1];
                let value = html_escape::decode_html_entities(&field_cap[2]).to_string();
                match name {
                    "dctitle" => title = value,
                    "dclink" => link = value,
                    "dcdescription" => {
                        content = if value.len() > 300 {
                            format!("{}...", &value[..300])
                        } else {
                            value
                        };
                    }
                    _ => {}
                }
            }

            if !link.is_empty() && !title.is_empty() {
                let mut result = SearchResult::new(title, link, content, "BASE".to_string());
                result.category = SearchCategory::General.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
