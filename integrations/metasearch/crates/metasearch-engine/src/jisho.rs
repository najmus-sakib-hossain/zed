//! Jisho — Japanese-English dictionary search.
//!
//! Queries the Jisho.org JSON API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct Jisho {
    metadata: EngineMetadata,
    client: Client,
}

impl Jisho {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "jisho".to_string().into(),
                display_name: "Jisho".to_string().into(),
                homepage: "https://jisho.org".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct JishoResponse {
    data: Vec<JishoEntry>,
}

#[derive(Deserialize)]
struct JishoEntry {
    slug: String,
    japanese: Vec<JishoJapanese>,
    senses: Vec<JishoSense>,
}

#[derive(Deserialize)]
struct JishoJapanese {
    word: Option<String>,
    reading: Option<String>,
}

#[derive(Deserialize)]
struct JishoSense {
    english_definitions: Vec<String>,
}

#[async_trait]
impl SearchEngine for Jisho {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://jisho.org/api/v1/search/words?keyword={}",
            urlencoding::encode(&query.query)
        );

        let resp: JishoResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let results = resp
            .data
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let title_parts: Vec<String> = entry
                    .japanese
                    .iter()
                    .map(|j| match (&j.word, &j.reading) {
                        (Some(w), Some(r)) => format!("{} ({})", w, r),
                        (Some(w), None) => w.clone(),
                        (None, Some(r)) => r.clone(),
                        (None, None) => String::new(),
                    })
                    .collect();
                let title = title_parts.join(", ");

                let defs: Vec<String> = entry
                    .senses
                    .iter()
                    .map(|s| s.english_definitions.join("; "))
                    .collect();
                let snippet = defs.join(". ");
                let snippet = if snippet.chars().count() > 300 {
                    let truncated: String = snippet.chars().take(300).collect();
                    format!("{}...", truncated)
                } else {
                    snippet
                };

                let page_url = format!("https://jisho.org/word/{}", entry.slug);
                let mut r = SearchResult::new(title, page_url, snippet, self.metadata.name.clone());
                r.engine_rank = (i + 1) as u32;
                r
            })
            .collect();

        Ok(results)
    }
}
