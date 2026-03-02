//! wttr.in weather engine implementation.
//!
//! JSON API: <https://wttr.in/{query}?format=j1>
//! Website: https://wttr.in
//! Features: Current weather conditions, no pagination

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Wttr {
    metadata: EngineMetadata,
    client: Client,
}

impl Wttr {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "wttr".to_string().into(),
                display_name: "wttr.in".to_string().into(),
                homepage: "https://wttr.in".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map wttr.in weather code to a human-readable condition string.
    /// Falls back to `weatherDesc[0].value` from the JSON if available.
    fn weather_condition(code: &str) -> &'static str {
        match code {
            "113" => "Clear/Sunny",
            "116" => "Partly cloudy",
            "119" => "Cloudy",
            "122" => "Overcast",
            "143" => "Mist",
            "176" => "Patchy rain",
            "179" => "Patchy snow",
            "182" => "Patchy sleet",
            "185" => "Patchy freezing drizzle",
            "200" => "Thundery outbreaks",
            "227" => "Blowing snow",
            "230" => "Blizzard",
            "248" => "Fog",
            "260" => "Freezing fog",
            "263" => "Patchy light drizzle",
            "266" => "Light drizzle",
            "281" => "Freezing drizzle",
            "284" => "Heavy freezing drizzle",
            "293" => "Patchy light rain",
            "296" => "Light rain",
            "299" => "Moderate rain",
            "302" => "Heavy rain",
            "305" => "Heavy rain at times",
            "308" => "Heavy rain",
            "311" => "Light freezing rain",
            "314" => "Heavy freezing rain",
            "317" => "Light sleet",
            "320" => "Heavy sleet",
            "323" => "Patchy light snow",
            "326" => "Light snow",
            "329" => "Patchy moderate snow",
            "332" => "Moderate snow",
            "335" => "Patchy heavy snow",
            "338" => "Heavy snow",
            "350" => "Ice pellets",
            "353" => "Light rain shower",
            "356" => "Heavy rain shower",
            "359" => "Torrential rain shower",
            "362" => "Light sleet showers",
            "365" => "Heavy sleet showers",
            "368" => "Light snow showers",
            "371" => "Heavy snow showers",
            "374" => "Light ice pellet showers",
            "377" => "Heavy ice pellet showers",
            "386" => "Patchy light rain with thunder",
            "389" => "Heavy rain with thunder",
            "392" => "Patchy light snow with thunder",
            "395" => "Heavy snow with thunder",
            _ => "Unknown",
        }
    }
}

#[async_trait]
impl SearchEngine for Wttr {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://wttr.in/{}?format=j1&lang=en",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await;

        // wttr.in may be slow or unreachable — return empty gracefully
        let resp = match resp {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        // Handle any non-success response (e.g., 404 = unknown location, 500 = server error)
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match resp
            .json()
            .await
        {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        let current = match data
            .get("current_condition")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
        {
            Some(c) => c,
            None => return Ok(results),
        };

        let temp_c = current
            .get("temp_C")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let feels_like = current
            .get("FeelsLikeC")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let wind_speed = current
            .get("windspeedKmph")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let humidity = current
            .get("humidity")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let pressure = current
            .get("pressure")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let weather_code = current
            .get("weatherCode")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        // Try to use weatherDesc[0].value first, fall back to code mapping
        let condition = current
            .get("weatherDesc")
            .and_then(|w| w.as_array())
            .and_then(|a| a.first())
            .and_then(|d| d.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| Self::weather_condition(weather_code));

        let title = format!(
            "Weather in {}: {}°C, {}",
            query.query, temp_c, condition
        );
        let content = format!(
            "Feels like {}°C | Wind: {} km/h | Humidity: {}% | Pressure: {} hPa",
            feels_like, wind_speed, humidity, pressure
        );
        let result_url = format!("https://wttr.in/{}", urlencoding::encode(&query.query));

        let mut r = SearchResult::new(&title, &result_url, &content, "wttr");
        r.engine_rank = 1;
        r.category = SearchCategory::General.to_string();
        results.push(r);

        Ok(results)
    }
}
