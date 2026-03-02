//! MyMemory Translation engine implementation.
//!
//! JSON API: <https://api.mymemory.translated.net/get>
//! Website: https://mymemory.translated.net
//! Features: Translation via MyMemory API, no pagination

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

pub struct Translated {
    metadata: EngineMetadata,
    client: Client,
}

impl Translated {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "translated".to_string().into(),
                display_name: "MyMemory".to_string().into(),
                homepage: "https://mymemory.translated.net".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Translated {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://api.mymemory.translated.net/get?q={}&langpair=auto|en",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        // Parse main translation from responseData.translatedText
        if let Some(translated_text) = data
            .get("responseData")
            .and_then(|rd| rd.get("translatedText"))
            .and_then(|t| t.as_str())
        {
            if !translated_text.is_empty() {
                let mut r = SearchResult::new(
                    format!("Translation: {}", translated_text),
                    "https://mymemory.translated.net/",
                    translated_text,
                    "translated",
                );
                r.engine_rank = 1;
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        // Parse additional translations from matches[]
        if let Some(matches) = data.get("matches").and_then(|m| m.as_array()) {
            for (i, m) in matches.iter().skip(1).take(4).enumerate() {
                let translation = m
                    .get("translation")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                let segment = m
                    .get("segment")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();

                if translation.is_empty() {
                    continue;
                }

                let source_lang = m
                    .get("source")
                    .and_then(|s| s.as_str())
                    .unwrap_or("auto");
                let target_lang = m
                    .get("target")
                    .and_then(|t| t.as_str())
                    .unwrap_or("en");

                let content = if !segment.is_empty() {
                    format!("{} → {} ({} → {})", segment, translation, source_lang, target_lang)
                } else {
                    translation.to_string()
                };

                let mut r = SearchResult::new(
                    format!("Translation: {}", translation),
                    "https://mymemory.translated.net/",
                    &content,
                    "translated",
                );
                r.engine_rank = (i + 2) as u32;
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        Ok(results)
    }
}
