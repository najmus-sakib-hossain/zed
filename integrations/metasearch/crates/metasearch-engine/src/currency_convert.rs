//! Currency Convert — via DuckDuckGo's spice API (JSONP)
//!
//! Fetches conversion rate from DDG's currency spice endpoint:
//! `https://duckduckgo.com/js/spice/currency/1/{FROM}/{TO}`
//!
//! The query should be like "100 USD to EUR" — we parse amount, from, to.
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/currency_convert.py>

use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct CurrencyConvert {
    client: Client,
}

impl CurrencyConvert {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for CurrencyConvert {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "CurrencyConvert".to_string().into(),
            display_name: "CurrencyConvert".to_string().into(),
            homepage: "https://duckduckgo.com".to_string().into(),
            categories: smallvec![SearchCategory::General, SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // Parse query like "100 USD to EUR" or "USD to EUR"
        let re = Regex::new(r"(?i)(\d+\.?\d*)\s*([A-Z]{3})\s+(?:to|in)\s+([A-Z]{3})").unwrap();
        let re_simple = Regex::new(r"(?i)([A-Z]{3})\s+(?:to|in)\s+([A-Z]{3})").unwrap();

        let (amount, from, to) = if let Some(caps) = re.captures(&query.query) {
            (
                caps[1].parse::<f64>().unwrap_or(1.0),
                caps[2].to_uppercase(),
                caps[3].to_uppercase(),
            )
        } else if let Some(caps) = re_simple.captures(&query.query) {
            (1.0, caps[1].to_uppercase(), caps[2].to_uppercase())
        } else {
            return Ok(vec![]);
        };

        let url = format!("https://duckduckgo.com/js/spice/currency/1/{}/{}", from, to);

        let resp =
            self.client.get(&url).send().await.map_err(|e| {
                MetasearchError::Engine(format!("CurrencyConvert request error: {e}"))
            })?;

        let text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("CurrencyConvert read error: {e}")))?;

        // JSONP response: first line is callback, JSON is between first \n and last \n
        let json_str = if let Some(start) = text.find('\n') {
            if let Some(end) = text.rfind('\n') {
                if end > start {
                    &text[start + 1..end - 2] // strip trailing `);"
                } else {
                    return Ok(vec![]);
                }
            } else {
                return Ok(vec![]);
            }
        } else {
            return Ok(vec![]);
        };

        let json: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| MetasearchError::Engine(format!("CurrencyConvert JSON error: {e}")))?;

        let rate = json
            .get("to")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("mid"))
            .and_then(|v| v.as_f64());

        let rate = match rate {
            Some(r) => r,
            None => return Ok(vec![]),
        };

        let converted = amount * rate;
        let answer = format!(
            "{} {} = {:.4} {} (1 {} = {:.6} {})",
            amount, from, converted, to, from, rate, to
        );

        let ddg_url = format!("https://duckduckgo.com/?q={}+to+{}", from, to);

        let mut result = SearchResult::new(
            format!("{} {} → {} {}", amount, from, converted, to),
            ddg_url,
            answer,
            "currency_convert",
        );
        result.engine_rank = 1;
        Ok(vec![result])
    }
}
