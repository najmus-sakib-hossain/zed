//! Material Icons — Google Material Symbols icon search.
//!
//! Fetches the full icon metadata JSON from Google Fonts and filters locally.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct MaterialIcons {
    metadata: EngineMetadata,
    client: Client,
}

impl MaterialIcons {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "material_icons".to_string().into(),
                display_name: "Material Icons".to_string().into(),
                homepage: "https://fonts.google.com/icons".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for MaterialIcons {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Use Iconify's search API — much faster than downloading the full
        // Google Fonts metadata file (~3 MB).
        let url = format!(
            "https://api.iconify.design/search?query={}&prefix=material-symbols&limit=20",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(icons) = json.get("icons").and_then(|v| v.as_array()) {
            for icon_ref in icons {
                let full_name = match icon_ref.as_str() {
                    Some(s) => s,
                    None => continue,
                };

                // full_name looks like "material-symbols:add" or "material-symbols:add-outlined"
                let name = full_name.split(':').nth(1).unwrap_or(full_name);
                let display = name.replace('-', " ");

                let page_url = format!(
                    "https://fonts.google.com/icons?icon.query={}",
                    urlencoding::encode(name)
                );
                let img_src = format!(
                    "https://api.iconify.design/material-symbols/{name}.svg"
                );

                let snippet = format!("Material Symbol icon: {}", display);

                let mut result = SearchResult::new(
                    display,
                    page_url,
                    snippet,
                    self.metadata.name.clone(),
                );
                result.engine_rank = (results.len() + 1) as u32;
                result.thumbnail = Some(img_src);
                result.category = SearchCategory::Images.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
