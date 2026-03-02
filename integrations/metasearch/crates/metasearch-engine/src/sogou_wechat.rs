//! Sogou WeChat article search engine implementation.
//!
//! Scrapes Sogou WeChat HTML results for public WeChat articles.
//! Website: https://weixin.sogou.com
//! Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

pub struct SogouWechat {
    metadata: EngineMetadata,
    client: Client,
}

impl SogouWechat {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "sogou_wechat".to_string().into(),
                display_name: "Sogou WeChat".to_string().into(),
                homepage: "https://weixin.sogou.com".to_string().into(),
                categories: smallvec![SearchCategory::News],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for SogouWechat {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page.max(1);
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://weixin.sogou.com/weixin?query={}&page={}&type=2",
            encoded, page
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&body);

        let li_sel =
            Selector::parse("li[id*='sogou_vr_']").map_err(|e| {
                MetasearchError::ParseError(format!("Selector parse error: {:?}", e))
            })?;
        let title_sel = Selector::parse("h3 a").map_err(|e| {
            MetasearchError::ParseError(format!("Selector parse error: {:?}", e))
        })?;
        let content_sel = Selector::parse("p.txt-info").map_err(|e| {
            MetasearchError::ParseError(format!("Selector parse error: {:?}", e))
        })?;

        let mut results = Vec::new();

        for (i, li) in document.select(&li_sel).enumerate() {
            let title_el = match li.select(&title_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title: String = title_el.text().collect::<String>().trim().to_string();
            let item_url = title_el
                .value()
                .attr("href")
                .unwrap_or_default()
                .to_string();

            if title.is_empty() || item_url.is_empty() {
                continue;
            }

            let content: String = li
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let mut r = SearchResult::new(&title, &item_url, &content, "sogou_wechat");
            r.engine_rank = i as u32;
            r.category = SearchCategory::News.to_string();
            results.push(r);
        }

        Ok(results)
    }
}
