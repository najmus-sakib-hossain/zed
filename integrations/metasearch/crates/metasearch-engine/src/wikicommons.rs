//! Wikimedia Commons search engine implementation.
//!
//! Translated from SearXNG's `wikicommons.py` (JSON API).
//! Wikimedia Commons hosts 120M+ freely usable media files.
//! Website: https://commons.wikimedia.org
//! Features: Paging (image search via MediaWiki API)

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
use tracing::info;
use smallvec::smallvec;

pub struct WikiCommons {
    metadata: EngineMetadata,
    client: Client,
}

impl WikiCommons {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "wikicommons".to_string().into(),
                display_name: "Wikimedia Commons".to_string().into(),
                homepage: "https://commons.wikimedia.org".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

/// MediaWiki API response structures.
#[derive(Deserialize, Debug)]
struct MwApiResponse {
    query: Option<MwQuery>,
}

#[derive(Deserialize, Debug)]
struct MwQuery {
    pages: Option<std::collections::HashMap<String, MwPage>>,
}

#[derive(Deserialize, Debug)]
struct MwPage {
    title: Option<String>,
    #[serde(default)]
    imageinfo: Vec<MwImageInfo>,
}

#[derive(Deserialize, Debug)]
struct MwImageInfo {
    url: Option<String>,
    descriptionurl: Option<String>,
    thumburl: Option<String>,
    mime: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    size: Option<u64>,
}

#[async_trait]
impl SearchEngine for WikiCommons {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let results_per_page: u32 = 10;
        let page = query.page.max(1);
        let offset = results_per_page * (page - 1);
        let _encoded = urlencoding::encode(&query.query);

        // Search for image files using MediaWiki generator search API
        let filetype = "bitmap|drawing";
        let gsrsearch = format!("filetype:{} {}", filetype, query.query);
        let gsrsearch_encoded = urlencoding::encode(&gsrsearch);

        let url = format!(
            "https://commons.wikimedia.org/w/api.php?\
            format=json&uselang=en&action=query\
            &prop=info%7Cimageinfo\
            &generator=search\
            &gsrnamespace=6\
            &gsrprop=snippet\
            &gsrlimit={}\
            &gsroffset={}\
            &gsrsearch={}\
            &iiprop=url%7Csize%7Cmime\
            &iiurlheight=180",
            results_per_page, offset, gsrsearch_encoded
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "metasearch/1.0 (https://github.com/najmus-sakib-hossain/metasearch)",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let data: MwApiResponse = serde_json::from_str(&body)
            .map_err(|e| MetasearchError::ParseError(format!("JSON parse error: {}", e)))?;

        let mut results = Vec::new();

        if let Some(query_data) = data.query {
            if let Some(pages) = query_data.pages {
                for (i, (_page_id, page)) in pages.iter().enumerate() {
                    if page.imageinfo.is_empty() {
                        continue;
                    }

                    let imageinfo = &page.imageinfo[0];

                    // Clean up title: remove "File:" prefix and file extension
                    let title = page
                        .title
                        .as_deref()
                        .unwrap_or_default()
                        .replace("File:", "");
                    let title = title
                        .rsplit_once('.')
                        .map(|(name, _ext)| name)
                        .unwrap_or(&title);

                    let description_url = imageinfo.descriptionurl.as_deref().unwrap_or_default();
                    let _media_url = imageinfo.url.as_deref().unwrap_or_default();

                    if title.is_empty() || description_url.is_empty() {
                        continue;
                    }

                    // Build content with resolution and mime info
                    let mut content_parts = Vec::new();
                    if let (Some(w), Some(h)) = (imageinfo.width, imageinfo.height) {
                        content_parts.push(format!("{}x{}", w, h));
                    }
                    if let Some(mime) = &imageinfo.mime {
                        content_parts.push(mime.clone());
                    }
                    if let Some(size) = imageinfo.size {
                        content_parts.push(humanize_bytes(size));
                    }
                    let content = content_parts.join(" · ");

                    let mut r = SearchResult::new(title, description_url, &content, "wikicommons");
                    r.engine_rank = i as u32;
                    r.category = SearchCategory::Images.to_string();

                    // Set thumbnail if available
                    if let Some(thumb) = &imageinfo.thumburl {
                        r.thumbnail = Some(thumb.clone());
                    }

                    results.push(r);
                }
            }
        }

        info!(
            engine = "wikicommons",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}

/// Convert bytes to a human-readable string.
fn humanize_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
