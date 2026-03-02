//! NVD engine — search the National Vulnerability Database.

use async_trait::async_trait;
use chrono::DateTime;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

const PAGE_SIZE: u32 = 10;

pub struct Nvd {
    metadata: EngineMetadata,
    client: Client,
}

impl Nvd {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "nvd".to_string().into(),
                display_name: "NVD".to_string().into(),
                homepage: "https://nvd.nist.gov".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Nvd {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let offset = (query.page - 1) * PAGE_SIZE;

        let url = format!(
            "https://nvd.nist.gov/extensions/nudp/services/json/nvd/cve/search/results?resultType=records&keyword={}&rowCount={}&offset={}",
            urlencoding::encode(&query.query),
            PAGE_SIZE,
            offset,
        );

        let resp = self
            .client
            .get(&url)
            .header("Referer", "https://nvd.nist.gov/vuln/search")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        let vulnerabilities = data
            .get("response")
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("grid"))
            .and_then(|grid| grid.get("vulnerabilities"))
            .and_then(|v| v.as_array());

        if let Some(vulns) = vulnerabilities {
            for (i, vuln) in vulns.iter().enumerate() {
                let cve = &vuln["cve"];
                let cve_id = cve["id"].as_str().unwrap_or_default();

                if cve_id.is_empty() {
                    continue;
                }

                let vuln_url = format!("https://nvd.nist.gov/vuln/detail/{}", cve_id);

                let content = cve["descriptions"]
                    .as_array()
                    .and_then(|descs| descs.first())
                    .and_then(|d| d["value"].as_str())
                    .unwrap_or("")
                    .to_string();

                let mut result = SearchResult::new(
                    cve_id.to_string(),
                    vuln_url,
                    content,
                    "nvd".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();

                if let Some(published) = cve["published"].as_str() {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(published) {
                        result.published_date = Some(dt.into());
                    }
                }

                results.push(result);
            }
        }

        Ok(results)
    }
}
